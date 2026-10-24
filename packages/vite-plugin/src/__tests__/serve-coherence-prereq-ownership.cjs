const assert = require('node:assert/strict')
const { join, resolve } = require('node:path')
const { pathToFileURL } = require('node:url')
const { put } = require('./reservation-native-fixture.cjs')
const { observe } = require('./reservation-native-observer.cjs')
const { hooks } = require('./serve-coherence-async-fixture.cjs')
const wasm = require('@devup-ui/wasm')
const root = process.argv[4]
assert(root)
const telemetry = observe()
const [first = 'esm', second = 'esm'] = process.argv.slice(2)
/** @returns {Promise<{DevupUI:typeof import('../plugin').DevupUI}>} */
async function load(/** @type {string} */ format) {
  return format === 'esm'
    ? import(pathToFileURL(resolve(__dirname, '../../dist/index.mjs')).href)
    : require('../../dist/' + 'index.cjs')
}
async function main() {
  // Given: A owns nonempty real engine state under its own options.
  put(root, 'package.json', '{"name":"ownership-native","type":"module"}')
  put(root, 'devup.json', '{}')
  put(root, 'index.html', '<script type="module" src="/src/A.tsx"></script>')
  const source =
    "import {Box} from '@devup-ui/react'; export const A=()=> <Box probe='orchid'/>;"
  put(root, 'src/A.tsx', source)
  process.chdir(root)
  const owner = await load(first)
  const { createServer } = await import('vite')
  const options = {
    singleCss: true,
    devupFile: join(root, 'devup.json'),
    distDir: join(root, 'df'),
    cssDir: join(root, 'df/css'),
  }
  /** @type {PromiseWithResolvers<void>} */
  const ready = Promise.withResolvers()
  let configResolved = 0
  const server = await createServer({
    root,
    configFile: false,
    logLevel: 'error',
    oxc: { jsx: { runtime: 'classic' } },
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, fs: { strict: false } },
    plugins: [
      owner.DevupUI({
        ...options,
        prefix: 'WA',
        shorthands: { probe: ['color'] },
      }),
      {
        name: 'ownership-native-observer',
        configResolved() {
          configResolved++
        },
        configureServer(server) {
          server.watcher.once('ready', () => ready.resolve())
        },
      },
    ],
  })
  /** @type {ReturnType<typeof owner.DevupUI>} */
  let other
  try {
    await server.listen()
    await ready.promise
    await server.environments.client.transformRequest('/src/A.tsx')
    const before = telemetry.snapshot()
    const sheet = wasm.exportSheet()
    const cjsOwnership = require('@devup-ui/plugin-utils').getBuildOwnership
    const esmOwnership = (await import('@devup-ui/plugin-utils'))
      .getBuildOwnership
    const lease = cjsOwnership()
    assert.equal(lease.running, 1)
    assert.deepEqual(esmOwnership(), lease)
    const initial = telemetry.extractions.at(-1)
    assert(initial)
    const code = initial.code
    assert.notEqual(before.classMap, '{}')
    assert.match(before.css, /color:orchid/)
    assert.match(code, /WA/)
    // When: B is only constructed, including across module formats.
    const factory = await load(second)
    other = factory.DevupUI({
      ...options,
      prefix: 'WB',
      shorthands: { probe: ['backgroundColor'] },
    })
    const constructed = telemetry.snapshot()
    // Then: construction cannot wipe or mutate A's sheet/maps/options.
    assert.equal(
      constructed.classMap,
      before.classMap,
      'construction must preserve active class map',
    )
    assert.equal(constructed.fileMap, before.fileMap)
    assert.equal(constructed.css, before.css)
    assert.equal(constructed.count, before.count)
    assert.equal(wasm.exportSheet(), sheet)
    assert.deepEqual(cjsOwnership(), lease)
    assert.deepEqual(esmOwnership(), cjsOwnership())
    await server.environments.client.transformRequest(
      '/src/A.tsx?prereq=overlap',
    )
    const after = telemetry.snapshot()
    const output = telemetry.extractions.at(-1)
    assert(output)
    assert.equal(output.input, source)
    assert.equal(
      output.code,
      code,
      'A must retain its own prefix and shorthand semantics',
    )
    assert.equal(after.css, before.css)
    assert.equal(after.classMap, before.classMap)
    assert.equal(after.fileMap, before.fileMap)
    assert.equal(configResolved, 1)
    assert(Array.isArray(other))
    const otherPlugin = { ...other[0], transform: other[0].transform.handler }
    assert(hooks(otherPlugin))
    await otherPlugin.closeBundle()
    other = undefined
    assert.deepEqual(esmOwnership(), lease)
    console.info(JSON.stringify({ first, second, code, css: after.css }))
  } finally {
    if (Array.isArray(other)) {
      const otherPlugin = { ...other[0], transform: other[0].transform.handler }
      assert(hooks(otherPlugin))
      await otherPlugin.closeBundle()
    }
    await server.close()
    process.chdir(__dirname)
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
