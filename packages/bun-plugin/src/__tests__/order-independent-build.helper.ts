import assert from 'node:assert/strict'
import { basename, join, relative, resolve } from 'node:path'

import type { BunPlugin, OnLoadCallback, PluginBuilder } from 'bun'

const [fixture, schedule] = process.argv.slice(2)
assert(fixture && (schedule === 'AB' || schedule === 'BA'))
process.chdir(fixture)

// Import only after entering the isolated fixture: the plugin resolves df at import time.
const { DevupUI } = await import('../plugin')
const { getUnreservedClassSlotCount } = await import('@devup-ui/wasm')
const a = join(fixture, 'src/A.tsx')
const b = join(fixture, 'src/B.tsx')
const entrypoints = [a, b]
const schedules = {
  AB: [a, b],
  BA: [b, a],
} as const
const [first, second] = schedules[schedule]

// These accumulators record custody; they never supply source or replacement output.
const waiting = new Map<string, () => void>()
const dispatched: string[] = []
const transformed = new Map<string, string>()

function control(callback: OnLoadCallback): OnLoadCallback {
  return async (args) => {
    assert.equal(args.namespace, 'file')
    assert(
      !waiting.has(args.path),
      'each genuine source callback is dispatched once',
    )
    await new Promise<void>((release) => {
      waiting.set(args.path, release)
      // Both native callbacks must arrive before either production callback executes.
      if (waiting.size === 2) {
        const start = waiting.get(first)
        assert(start)
        start()
      }
    })
    dispatched.push(args.path)
    try {
      const result = await callback(args)
      assert(
        result && 'contents' in result && typeof result.contents === 'string',
      )
      transformed.set(basename(args.path), result.contents)
      return result
    } finally {
      // A real callback failure must also let the other callback finish the native build.
      if (args.path === first) {
        const next = waiting.get(second)
        assert(next)
        next()
      }
    }
  }
}

function decorate(build: PluginBuilder): PluginBuilder {
  const onLoad: PluginBuilder['onLoad'] = (options, callback) =>
    build.onLoad(options, (args) => {
      if (options.namespace !== undefined || !entrypoints.includes(args.path))
        return callback(args)
      return control(callback)(args)
    })
  return new Proxy(build, {
    get(target, key) {
      if (key === 'onLoad') return onLoad
      const value: unknown = Reflect.get(target, key, target)
      return typeof value === 'function' ? value.bind(target) : value
    },
  })
}

const genuine = DevupUI()
const observed: BunPlugin = {
  ...genuine,
  async setup(build) {
    await genuine.setup(decorate(build))
  },
}
const result = await Bun.build({
  root: fixture,
  entrypoints,
  target: 'browser',
  format: 'esm',
  external: ['react', 'react/jsx-runtime'],
  plugins: [observed],
})
assert(result.success, result.logs.map(String).join('\n'))
assert.deepEqual(dispatched, [first, second])
assert.equal(transformed.size, 2)

const artifacts = new Map<string, string>()
for (const artifact of result.outputs) {
  const path = relative(fixture, resolve(artifact.path)).replaceAll('\\', '/')
  artifacts.set(
    path,
    Buffer.from(await artifact.arrayBuffer()).toString('base64'),
  )
}
process.stdout.write(
  JSON.stringify({
    transformed: Object.fromEntries(transformed),
    artifacts: Object.fromEntries(artifacts),
    residual: getUnreservedClassSlotCount(),
  }),
)
