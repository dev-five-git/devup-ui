import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { captureProcess } from '../../../../test-harness/producer-process'
import { ProducerProtocol } from '../../../../test-harness/producer-protocol'

it('unregisters real Vite Outputs before final capture and the original late-code fence', async () => {
  // Given the original real watch scenarios and an early pass-through weak observer.
  const root = resolve(import.meta.dir, '../../../..')
  const temporary = mkdtempSync(join(tmpdir(), 'vite-output-disposal-'))
  const trace = join(temporary, 'outputs.jsonl')
  const observer = join(temporary, 'observer.test.mjs')
  const originalFence = ProducerProtocol.prototype.armEndFence
  let held = false
  try {
    writeFileSync(trace, '')
    writeFileSync(
      observer,
      `
import { appendFileSync } from 'node:fs'
const Native = globalThis.FinalizationRegistry
const pending = new Set()
let sequence = 0
let final = false
const emit = (record) => appendFileSync(${JSON.stringify(trace)}, JSON.stringify(record) + '\\n')
globalThis.FinalizationRegistry = new Proxy(Native, {
  construct(target, args, newTarget) {
    const tokens = new WeakMap()
    const records = new Map()
    const callback = args[0]
    const registry = Reflect.construct(target, [function (...values) {
      const id = records.get(values[0])
      if (id !== undefined) {
        records.delete(values[0])
        emit({ action: 'finalize', id, afterFinal: final })
      }
      return Reflect.apply(callback, this, values)
    }], newTarget)
    registry.register = new Proxy(registry.register, {
      apply(method, receiver, values) {
        const result = Reflect.apply(method, receiver, values)
        const stack = new Error().stack
        if (values[0].constructor.name === 'Output' && /bindings[\\\\/]devup-ui-wasm[\\\\/]pkg[\\\\/]index.js/.test(stack)) {
          const id = ++sequence
          records.set(values[1], id)
          tokens.set(values[2], { id, pointer: values[1] })
          pending.add(id)
          emit({ action: 'register', id, stack })
        }
        return result
      },
    })
    registry.unregister = new Proxy(registry.unregister, {
      apply(method, receiver, values) {
        const result = Reflect.apply(method, receiver, values)
        const record = tokens.get(values[0])
        if (result && record) {
          records.delete(record.pointer)
          tokens.delete(values[0])
          pending.delete(record.id)
          emit({ action: 'unregister', id: record.id, afterFinal: final })
        }
        return result
      },
    })
    return registry
  },
})
process.stdout.write = new Proxy(process.stdout.write, {
  apply(method, receiver, args) {
    const text = String(args[0])
    if (text.startsWith(process.env.DEVUP_PRODUCER_TOKEN + ' ') && text.includes('"phase":"final"')) {
      final = true
      emit({ action: 'final', pending: [...pending] })
    }
    return Reflect.apply(method, receiver, args)
  },
})
`,
    )
    const require = createRequire(join(root, 'package.json'))
    const preloads = [
      observer,
      join(root, 'bun.preload.ts'),
      join(root, 'bun.setup.ts'),
      require.resolve('bun-test-env-dom'),
    ]
    const config = join(temporary, 'bunfig.toml')
    writeFileSync(
      config,
      readFileSync(join(root, 'test-harness/bunfig.toml'), 'utf8').replace(
        /^preload = .*$/m,
        `preload = ${JSON.stringify(preloads.map((path) => path.replaceAll('\\', '/')))}`,
      ),
    )
    ProducerProtocol.prototype.armEndFence = async function (
      captures,
      preload,
    ) {
      await originalFence.call(this, captures, preload)
      await new Promise<void>((done) => setTimeout(done, 11000))
      held = true
    }
    // When native capture drives the unchanged three selected-blue CSS assertions.
    const result = await captureProcess({
      root,
      config,
      coverage: join(temporary, 'coverage'),
      group: 'runtime',
      files: [
        'packages/vite-plugin/src/__tests__/resolution-watch.test.ts',
        'packages/vite-plugin/src/__tests__/resolution-watch-environments.test.ts',
      ],
      executable: process.execPath,
    })
    const records: {
      readonly action: string
      readonly id?: number
      readonly pending?: readonly number[]
      readonly afterFinal?: boolean
    }[] = readFileSync(trace, 'utf8')
      .trim()
      .split('\n')
      .map((line) => JSON.parse(line))
    const registered = records
      .filter((record) => record.action === 'register')
      .map((record) => record.id)
    expect(result.value.status).toBe(0)
    expect(result.value.output).toContain('3 pass')
    expect(registered.length).toBeGreaterThan(0)
    expect(
      records
        .filter((record) => record.action === 'unregister')
        .map((record) => record.id),
    ).toEqual(registered)
    expect(
      records
        .filter((record) => record.action === 'final')
        .map((record) => record.pending),
    ).toEqual([[]])
    expect(records.filter((record) => record.action === 'finalize')).toEqual([])
    expect(held).toBe(true)
  } finally {
    ProducerProtocol.prototype.armEndFence = originalFence
    console.info('WASM Output lifetime evidence', readFileSync(trace, 'utf8'))
    rmSync(temporary, { recursive: true, force: true })
  }
}, 120000)
