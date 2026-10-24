const assert = require('node:assert/strict')
const { readFileSync, rmSync } = require('node:fs')
const { join, resolve } = require('node:path')
const { pathToFileURL } = require('node:url')
const { put } = require('./reservation-native-fixture.cjs')
const { observe } = require('./reservation-native-observer.cjs')
const {
  observeRefreshFailure,
  configFailure,
} = require('./config-refresh-observer.cjs')
const wasm = require('@devup-ui/wasm')
const key = (/** @type {string} */ path) => resolve(path).replaceAll('\\', '/')
const mode = process.argv[2]
assert(['theme', 'alias'].includes(mode))
const root = process.argv[3]
assert(root)
const write = (/** @type {string} */ path, /** @type {string} */ text) =>
  put(root, path, text)
const telemetry = observe()
const theme = (/** @type {readonly (string|null)[]} */ length) =>
  JSON.stringify({ theme: { length: { default: { size: length } } } })
const alias = (/** @type {string} */ color) =>
  JSON.stringify({
    compilerOptions: {
      baseUrl: '.',
      paths: { '@tone': ['./src/' + color + '.js'] },
    },
  })

async function main() {
  // Given: both suppliers exist; only metadata changes after configuration.
  write('package.json', '{"name":"prereq-native","type":"module"}')
  write(
    'devup.json',
    mode === 'theme' ? '{"extends":["./config/base.json"]}' : '{}',
  )
  write('config/base.json', theme(['11px']))
  write('tsconfig.json', alias('red'))
  write('src/red.js', "export const tone='red'")
  write('src/blue.js', "export const tone='blue'")
  const source =
    mode === 'theme'
      ? 'import {Box} from \'@devup-ui/react\'; export const A=()=> <Box w="$size"/>;'
      : "import {Box} from '@devup-ui/react'; import {tone} from '@tone'; import './red.js'; import './blue.js'; export const A=()=> <Box color={tone}/>;"
  write('src/A.tsx', source)
  write('index.html', '<script type="module" src="/src/A.tsx"></script>')
  process.chdir(root)
  /** @type {{DevupUI:typeof import('../plugin').DevupUI}} */
  const { DevupUI } = await import(
    pathToFileURL(resolve(__dirname, '../../dist/index.mjs')).href
  )
  const { createServer } = await import('vite')
  let configResolved = 0
  /** @type {PromiseWithResolvers<void>} */
  const ready = Promise.withResolvers()
  /** @type {Map<string,PromiseWithResolvers<void>>} */
  const notifications = new Map()
  /** @type {Set<string>} */
  const registrations = new Set()
  /** @type {Map<string, unknown>} */
  const failures = new Map()
  const plugins = DevupUI({
    singleCss: true,
    devupFile: join(root, 'devup.json'),
    distDir: join(root, 'df'),
    cssDir: join(root, 'df/css'),
  })
  assert(Array.isArray(plugins))
  observeRefreshFailure(plugins[0], (fact, environment, error) => {
    const file = fact.phase === 'watchChange' ? fact.file : fact.context.file
    console.info(
      JSON.stringify({
        event: 'config-refresh-failure',
        phase: fact.phase,
        file,
        environment,
      }),
    )
    if (environment !== 'client') return
    failures.set(key(file), error)
    notifications.get(key(file))?.resolve()
  })
  const server = await createServer({
    root,
    configFile: false,
    logLevel: 'error',
    oxc: { jsx: { runtime: 'classic' } },
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, fs: { strict: false } },
    plugins: [
      {
        name: 'prereq-native-observer',
        enforce: 'pre',
        configResolved() {
          configResolved++
        },
        configureServer(server) {
          const add = server.watcher.add.bind(server.watcher)
          server.watcher.add = (paths) => {
            for (const path of typeof paths === 'string' ? [paths] : paths)
              registrations.add(key(path))
            return add(paths)
          }
          server.watcher.once('ready', () => ready.resolve())
        },
        hotUpdate: {
          order: 'post',
          handler({ file }) {
            if (this.environment.name === 'client')
              notifications.get(key(file))?.resolve()
          },
        },
      },
      plugins,
    ],
  })
  async function extract(/** @type {string} */ query) {
    const count = telemetry.extractions.length
    const result = await server.environments.client.transformRequest(
      '/src/A.tsx' + query,
    )
    assert(result)
    const output = telemetry.extractions
      .slice(count)
      .findLast((row) => row.id === key(join(root, 'src/A.tsx')))
    assert(output, 'must actually extract unchanged consumer')
    return {
      output,
      css: wasm.getCss(null, false),
      rows: telemetry.reservations.at(-1)?.rows,
    }
  }
  async function change(
    /** @type {string} */ path,
    /** @type {()=>void} */ operation,
    expectedFailure = false,
  ) {
    /** @type {PromiseWithResolvers<void>} */
    const done = Promise.withResolvers()
    const file = key(join(root, path))
    failures.delete(file)
    notifications.set(file, done)
    const native = new Promise((resolveEvent) => {
      const listener = (
        /** @type {string} */ event,
        /** @type {string} */ changed,
      ) => {
        if (key(changed) !== file) return
        server.watcher.off('all', listener)
        resolveEvent(event)
      }
      server.watcher.on('all', listener)
    })
    operation()
    const [event] = await Promise.all([native, done.promise])
    notifications.delete(file)
    const failure = failures.get(file)
    if (!expectedFailure) assert.equal(failure, undefined)
    return { failure, event }
  }
  try {
    await server.listen()
    await ready.promise
    const before = await extract('')
    const changed = mode === 'theme' ? 'config/base.json' : 'tsconfig.json'
    // When: a real filesystem event reaches the unchanged configured plugin.
    await change(changed, () =>
      write(
        changed,
        mode === 'theme' ? theme(['11px', null, '29px']) : alias('blue'),
      ),
    )
    const after = await extract('?prereq=metadata')
    // Then: inventory and consumer bytes stay fixed, but real output reflects metadata.
    assert.equal(readFileSync(join(root, 'src/A.tsx'), 'utf8'), source)
    assert.deepEqual(after.rows, before.rows)
    assert.equal(configResolved, 1)
    if (mode === 'theme') {
      await change('devup.json', () =>
        write(
          'devup.json',
          '{"extends":["./config/base.json"],"control":true}',
        ),
      )
      const control = await extract('?prereq=root-refresh')
      assert.notEqual(control.output.code, before.output.code)
      assert.match(control.css, /29px/)
      assert.match(control.css, /@media/)
      assert.match(
        after.css,
        /29px/,
        'inherited theme edit must apply responsive token expansion',
      )
      assert.notEqual(after.output.code, before.output.code)
      assert(registrations.has(key(join(root, changed))))
      const deleted = await change(
        changed,
        () => rmSync(join(root, changed)),
        true,
      )
      assert.equal(deleted.event, 'unlink')
      configFailure(join(root, changed))(deleted.failure)
      const count = telemetry.extractions.length
      await assert.rejects(
        extract('?prereq=deleted'),
        configFailure(join(root, changed)),
      )
      assert.equal(telemetry.extractions.length, count)
      const repaired = await change(changed, () =>
        write(changed, theme(['17px', null, '37px'])),
      )
      assert.equal(repaired.event, 'add')
      const recreated = await extract('?prereq=recreated')
      assert.match(recreated.css, /37px/)
      assert.match(recreated.css, /@media/)
      const malformed = await change(changed, () => write(changed, '{'), true)
      assert.equal(malformed.event, 'change')
      configFailure(join(root, changed), 'SyntaxError')(malformed.failure)
      await assert.rejects(
        extract('?prereq=malformed'),
        configFailure(join(root, changed), 'SyntaxError'),
      )
      const removedMalformed = await change(
        changed,
        () => rmSync(join(root, changed)),
        true,
      )
      assert.equal(removedMalformed.event, 'unlink')
      configFailure(join(root, changed))(removedMalformed.failure)
      await change(changed, () => write(changed, theme(['19px', null, '47px'])))
      const valid = await extract('?prereq=repaired')
      assert.match(valid.css, /47px/)
      assert.match(valid.css, /@media/)
      assert.equal(configResolved, 1)
    } else {
      assert.match(before.css, /color:red/)
      assert(
        before.output.dependencies.some((path) => path.endsWith('/red.js')),
      )
      assert(
        after.output.dependencies.some((path) => path.endsWith('/blue.js')),
        'live resolver must select blue supplier after paths edit',
      )
      assert.match(after.output.css ?? '', /color:blue/)
    }
    console.info(JSON.stringify({ mode, configResolved, before, after }))
  } finally {
    await server.close()
    process.chdir(__dirname)
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
