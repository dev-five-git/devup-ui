const assert = require('node:assert/strict')
const { mkdirSync, writeFileSync } = require('node:fs')
const { basename, dirname, resolve } = require('node:path')
const { observe } = require('./reservation-native-observer.cjs')

/** @param {string | undefined} value */
function required(value) {
  assert(value)
  return value
}
const root = resolve(required(process.argv[2]))
const scenario = required(process.argv[3])
const order = required(process.argv[4])
const receipt = required(process.argv[5])
/** @param {string} path @param {string} code */
function put(path, code) {
  const file = resolve(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, code)
}
/** @param {string} color */
function style(color) {
  return `import {css} from '@devup-ui/react'; console.log(css({color:'${color}'}))`
}
put('devup.json', '{}')
put('package.json', '{"type":"module"}')
put(
  'index.html',
  scenario === 'classic'
    ? "<!doctype html>\n<html><head></head><body><script>import('./outside/A.js');import('./outside/B.js')</script></body></html>"
    : '<script type="module" src="/src/Initial.js"></script>',
)
put('src/Initial.js', style('maroon'))
put('outside/A.js', style('red'))
put('outside/B.js', style('blue'))
process.chdir(root)
const telemetry = observe()
/** @type {{DevupUI:(options:{distDir:string,cssDir:string,devupFile:string,singleCss:boolean})=>import('vite').PluginOption}} */
const { DevupUI } = require('../../dist/' + 'index.cjs')
const gate = Promise.withResolvers()
telemetry.onExtract((id) => {
  if (basename(id) === `${order[0]}.js`) gate.resolve(undefined)
})
/** @type {Record<string, unknown>} */
const details = {}
/** @param {string} phase */
function record(phase) {
  writeFileSync(
    receipt,
    JSON.stringify(
      { phase, scenario, order, root, details, ...telemetry.snapshot() },
      null,
      2,
    ),
  )
}
async function main() {
  const { build, createServer, version } = await import('vite')
  details.version = version
  /** @type {import('vite').InlineConfig} */
  const config = {
    configFile: false,
    root,
    logLevel: 'error',
    plugins: [
      {
        name: 'physical-arrival',
        enforce: 'pre',
        async transform(_code, id) {
          if (scenario === 'classic' && basename(id) === `${order[1]}.js`)
            await gate.promise
        },
      },
      DevupUI({
        distDir: resolve(root, 'df'),
        cssDir: scenario === 'classic' ? resolve(root, 'df/css') : root,
        devupFile: resolve(root, 'devup.json'),
        singleCss: true,
      }),
    ],
    build: { outDir: 'out', minify: false, sourcemap: true },
    server: { host: '127.0.0.1', port: 0 },
    optimizeDeps: { noDiscovery: true, include: [] },
  }
  if (scenario === 'classic') {
    try {
      const result = await build(config)
      assert(!Array.isArray(result) && 'output' in result)
      details.assets = Object.fromEntries(
        result.output.map((item) => [
          item.fileName,
          item.type === 'chunk' ? item.code : String(item.source),
        ]),
      )
      record('outcome-before-close')
      const arrivals = telemetry.extractions.filter((row) =>
        ['A.js', 'B.js'].includes(basename(row.id)),
      )
      assert.equal(
        arrivals.length,
        0,
        'native classic imports are assets, not compiled modules',
      )
      const html = result.output.find((item) => item.fileName === 'index.html')
      assert(html && html.type === 'asset')
      const urls = [
        ...String(html.source).matchAll(
          /data:text\/javascript;base64,([^']+)/g,
        ),
      ]
      assert.deepEqual(
        urls.map((match) =>
          Buffer.from(required(match[1]), 'base64').toString(),
        ),
        [style('red'), style('blue')],
      )
      assert(
        telemetry.reservations.every((batch) =>
          batch.rows.every(
            (row) => !['A.js', 'B.js'].includes(basename(row[0])),
          ),
        ),
      )
    } catch (error) {
      details.error = error instanceof Error ? error.message : String(error)
      record('error')
      throw error
    }
    return
  }
  const server = await createServer(config)
  const ready = Promise.withResolvers()
  server.watcher.once('ready', () => ready.resolve(undefined))
  try {
    await server.listen()
    await ready.promise
    const initial =
      await server.environments.client?.transformRequest('/src/Initial.js')
    assert(initial?.code.includes('console.log'))
    assert.equal(telemetry.snapshot().count, 0)
    const additions = Promise.withResolvers()
    /** @type {Set<string>} */
    const seen = new Set()
    server.watcher.on('add', (file) => {
      if (['A.js', 'B.js'].includes(basename(file))) {
        seen.add(basename(file))
        if (seen.size === 2) additions.resolve(undefined)
      }
    })
    const before = telemetry.reservations.length
    put('src/B.js', style('blue'))
    put('src/A.js', style('red'))
    await additions.promise
    details.watchAdds = [...seen]
    for (const name of order) {
      const result = await server.environments.client?.transformRequest(
        `/src/${name}.js`,
      )
      assert(result?.code.includes('console.log'))
    }
    details.reservationsBefore = before
    record('outcome-before-close')
    assert.equal(
      telemetry.snapshot().count,
      0,
      'cssDir=root new-source physical residual',
    )
    assert(
      telemetry.reservations
        .slice(before)
        .some((batch) =>
          ['A.js', 'B.js'].every((name) =>
            batch.rows.some(
              (row) =>
                row[0] === resolve(root, 'src', name).replaceAll('\\', '/'),
            ),
          ),
        ),
    )
  } catch (error) {
    details.error = error instanceof Error ? error.message : String(error)
    record('error-before-close')
    throw error
  } finally {
    await server.close()
    record('close-exit')
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
