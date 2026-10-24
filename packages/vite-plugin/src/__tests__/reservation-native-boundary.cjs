const assert = require('node:assert/strict')
const {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
} = require('node:fs')
const { tmpdir } = require('node:os')
const { dirname, join, resolve } = require('node:path')
const { put } = require('./reservation-native-fixture.cjs')
const { observe } = require('./reservation-native-observer.cjs')

async function main() {
  const root = realpathSync(
    mkdtempSync(join(tmpdir(), 'devup-watch-boundary-')),
  )
  const write = (/** @type {string} */ path, /** @type {string} */ code) =>
    put(root, path, code)
  write('package.json', '{"name":"boundary-native","type":"module"}')
  write('devup.json', '{}')
  write('index.html', '<script type="module" src="/src/A.tsx"></script>')
  write(
    'src/A.tsx',
    "import {Box} from '@devup-ui/react'; import {tone} from 'accepted'; import '../workspace/accepted/dist/red.js'; import '../workspace/accepted/dist/blue.js'; export const A=()=> <Box color={tone}/>;",
  )
  write(
    'workspace/accepted/package.json',
    '{"name":"accepted","type":"module","exports":"./dist/red.js"}',
  )
  write(
    'workspace/accepted/dist/package.json',
    '{"name":"nearer-dist","type":"module"}',
  )
  write('workspace/accepted/dist/red.js', "export const tone='red'")
  write('workspace/accepted/dist/blue.js', "export const tone='blue'")
  mkdirSync(join(root, 'node_modules'))
  symlinkSync(
    join(root, 'workspace/accepted'),
    join(root, 'node_modules/accepted'),
    'junction',
  )
  process.chdir(root)
  const telemetry = observe()
  /** @type {{DevupUI: typeof import('../index').DevupUI}} */
  const { DevupUI } = require('../../dist/' + 'index.cjs')
  const { createServer } = await import('vite')
  /** @type {PromiseWithResolvers<void>} */
  const ready = Promise.withResolvers()
  /** @type {Set<string>} */
  const registered = new Set()
  /** @type {Map<string, PromiseWithResolvers<void>>} */
  const updates = new Map()
  const key = (/** @type {string} */ path) =>
    resolve(path).replaceAll('\\', '/')
  const server = await createServer({
    configFile: false,
    root,
    logLevel: 'error',
    oxc: { jsx: { runtime: 'classic' } },
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, fs: { strict: false } },
    plugins: [
      {
        name: 'boundary-native-observer',
        enforce: 'pre',
        configureServer(server) {
          const add = server.watcher.add.bind(server.watcher)
          server.watcher.add = (paths) => {
            for (const path of typeof paths === 'string' ? [paths] : paths)
              registered.add(key(path))
            return add(paths)
          }
          server.watcher.once('ready', () => ready.resolve())
        },
        hotUpdate: {
          order: 'post',
          handler({ file }) {
            if (this.environment.name === 'client')
              updates.get(key(file))?.resolve()
          },
        },
      },
      DevupUI({
        include: ['accepted'],
        singleCss: true,
        devupFile: join(root, 'devup.json'),
        distDir: join(root, 'df'),
        cssDir: join(root, 'df/css'),
      }),
    ],
  })
  /** @type {{root:string,events:string[],registered:string[],reservations?:number}} */
  const evidence = { root, events: [], registered: [] }
  try {
    await server.listen()
    await ready.promise
    await server.environments.client.transformRequest('/src/A.tsx')
    const above = key(join(dirname(root), 'package.json'))
    assert(
      !registered.has(above),
      'cold named fixture must not register shared Temp manifest',
    )
    const metadata = 'workspace/accepted/package.json'
    assert(
      registered.has(key(join(root, metadata))),
      'physical package-entry manifest retained above dist scope',
    )
    assert(
      registered.has(key(join(root, 'src/package.json'))),
      'closer missing scope retained',
    )
    const initial = telemetry.reservations.at(-1)?.rows
    assert(initial)
    /** @param {string} path @param {() => void} operation */
    async function change(path, operation) {
      /** @type {PromiseWithResolvers<void>} */
      const notification = Promise.withResolvers()
      updates.set(key(join(root, path)), notification)
      operation()
      await notification.promise
      updates.delete(key(join(root, path)))
      evidence.events.push(path)
      const before = telemetry.reservations.length
      await server.environments.client.transformRequest(
        `/src/A.tsx?boundary=${evidence.events.length}`,
      )
      assert(
        telemetry.reservations.length > before,
        'native metadata event invalidates preparation',
      )
    }
    await change('src/package.json', () =>
      write('src/package.json', '{"name":"closer","type":"module"}'),
    )
    await change('src/package.json', () =>
      rmSync(join(root, 'src/package.json')),
    )
    await change('src/package.json', () =>
      write('src/package.json', '{"name":"closer","type":"module"}'),
    )
    await change(metadata, () =>
      write(
        metadata,
        '{"name":"accepted","type":"module","exports":"./dist/blue.js"}',
      ),
    )
    assert.deepEqual(
      telemetry.reservations.at(-1)?.rows,
      initial,
      'exports switch keeps identical source inventory',
    )
    await change('workspace/accepted/dist/blue.js', () =>
      write('workspace/accepted/dist/blue.js', "export const tone='navy'"),
    )
    assert(
      telemetry.reservations
        .at(-1)
        ?.rows.some((row) => row[1] === "export const tone='navy'"),
    )
    evidence.registered = [...registered]
    evidence.reservations = telemetry.reservations.length
    console.info(JSON.stringify(evidence))
  } finally {
    await server.close()
    process.chdir(__dirname)
    rmSync(root, { recursive: true, force: true })
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
