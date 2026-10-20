const assert = require('node:assert/strict')
const { writeFileSync } = require('node:fs')
const { basename, resolve } = require('node:path')
const { observe } = require('./reservation-native-observer.cjs')
const { fixture } = require('./reservation-native-fixture.cjs')
const { runServer } = require('./reservation-native-server.cjs')
const { text } = require('./reservation-native-receipt.cjs')
const { checkHoist } = require('./reservation-native-hoist-checks.cjs')

const args = process.argv.slice(2)
const root = text(args[0]),
  scenario = text(args[1]),
  order = text(args[2]),
  mode = text(args[3]),
  single = text(args[4]),
  receipt = text(args[5])
process.chdir(root)
const project = fixture(root, scenario, mode)
const telemetry = observe()
/** @type {{DevupUI:(options:{distDir:string,cssDir:string,devupFile:string,singleCss:boolean,include:string[],atomHoist?:number})=>import('vite').PluginOption}} */
const { DevupUI } = require('../../dist/' + 'index.cjs')
const gate = Promise.withResolvers()
telemetry.onExtract((id) => {
  if (basename(id).endsWith(`${order[0]}.tsx`)) gate.resolve(undefined)
})
/** @type {Record<string,string>} */
const assets = {}
/** @type {Record<string,string>} */
const http = {}
/** @type {Record<string,unknown>} */
const details = {}
/** @type {{phase:string,extractions:number}[]} */
const phaseExtractionCounts = []
if (scenario === 'linked') details.phaseExtractionCounts = phaseExtractionCounts
const watcherReady = Promise.withResolvers()
/** @type {PromiseWithResolvers<number>} */
const nativeUpdate = Promise.withResolvers()
/** @param {string} phase */
function checkpoint(phase) {
  details.phase = phase
  if (scenario === 'linked')
    phaseExtractionCounts.push({
      phase,
      extractions: telemetry.extractions.length,
    })
  writeFileSync(
    receipt,
    JSON.stringify({ ...telemetry.snapshot(), assets, http, details }, null, 2),
  )
}

async function main() {
  const { build, createBuilder, version } = await import('vite')
  details.version = version
  /** @type {import('vite').Plugin} */
  const observer = {
    name: 'native-qualification',
    enforce: 'pre',
    configureServer(server) {
      server.watcher.once('ready', () => watcherReady.resolve(undefined))
    },
    hotUpdate: {
      order: 'post',
      handler({ file, modules, timestamp }) {
        if (
          file.endsWith('/supplier.js') ||
          file.endsWith('/workspace/accepted/dist/index.js')
        ) {
          details.nativeHmr = {
            file,
            timestamp,
            modules: modules.map((module) => module.id),
          }
          if (this.environment.name === 'client')
            nativeUpdate.resolve(timestamp)
        }
      },
    },
    async transform(_code, id) {
      if (
        basename(id).endsWith(`${order[1]}.tsx`) &&
        !['hmr', 'retained', 'history', 'linked'].includes(scenario)
      )
        await gate.promise
    },
    resolveId(id) {
      if (id === 'virtual:styled') return '\0virtual-styled.js'
    },
    load(id) {
      if (id === '\0virtual-styled.js')
        return "import {css} from '@devup-ui/react'; console.log(css({color:'chartreuse',padding:'79px'}))"
    },
    generateBundle(_options, bundle) {
      for (const item of Object.values(bundle))
        assets[`${this.environment.name}/${item.fileName}`] =
          item.type === 'chunk' ? item.code : String(item.source)
    },
  }
  /** @type {import('vite').InlineConfig} */
  const config = {
    configFile: false,
    root,
    base: project.base,
    logLevel: 'error',
    define: scenario === 'devfalse' ? { 'import.meta.env.DEV': false } : {},
    oxc: { jsx: { runtime: 'classic' } },
    plugins: [
      observer,
      DevupUI({
        distDir: resolve(root, 'df'),
        cssDir: resolve(root, 'df/devup-ui'),
        devupFile: resolve(root, 'devup.json'),
        singleCss: single === 'single',
        include: ['accepted'],
        ...(scenario === 'hoist-lazy' ? { atomHoist: 3 } : {}),
      }),
    ],
    build: {
      ...project.build,
      minify: false,
      sourcemap: true,
      outDir: 'out',
      emptyOutDir: scenario !== 'stale',
      rolldownOptions: {
        ...project.build.rolldownOptions,
        output: {
          entryFileNames: '[name].js',
          chunkFileNames: '[name].js',
          assetFileNames: '[name][extname]',
        },
      },
    },
    server: { host: '127.0.0.1', port: 0, fs: { strict: false } },
    optimizeDeps: { noDiscovery: true, include: [] },
  }
  try {
    if (scenario === 'environments') {
      config.environments = {
        rsc: {
          consumer: 'server',
          build: { ssr: 'server.tsx', outDir: 'out/rsc' },
        },
        client: { consumer: 'client', build: { outDir: 'out/client' } },
      }
      config.builder = {
        sharedPlugins: true,
        async buildApp(builder) {
          for (const name of order === 'AB'
            ? ['rsc', 'client']
            : ['client', 'rsc']) {
            const environment = builder.environments[name]
            assert(environment)
            await builder.build(environment)
          }
        },
      }
      const builder = await createBuilder(config)
      await builder.buildApp()
    } else if (mode === 'build') {
      await build(config)
    } else {
      await runServer(config, {
        scenario,
        root,
        order,
        paths: project.paths,
        base: project.base,
        telemetry,
        http,
        details,
        ready: watcherReady.promise,
        nativeUpdate: nativeUpdate.promise,
        checkpoint,
      })
    }
  } catch (error) {
    details.error =
      error instanceof Error
        ? { name: error.name, message: error.message, stack: error.stack }
        : String(error)
    if (scenario !== 'error') throw error
  } finally {
    writeFileSync(
      receipt,
      JSON.stringify(
        { ...telemetry.snapshot(), assets, http, details, version },
        null,
        2,
      ),
    )
  }
  const { reservations, extractions, count, css } = telemetry.snapshot()
  assert(reservations.length > 0)
  if (scenario === 'error') {
    assert(details.error)
    assert.match(JSON.stringify(details.error), /root\.js:1:\d+/)
    assert.match(JSON.stringify(details.error), /window\.reachable/)
    assert(!css.includes('window.reachable'))
    return
  }
  if (scenario === 'virtual') {
    assert(
      extractions.some(
        (row) => row.id.includes('virtual-styled') && row.after > row.before,
      ),
    )
    assert(count > 0)
    return
  }
  assert.equal(count, 0, `${scenario} physical residual`)
  assert(extractions.every((row) => row.before === 0 && row.after === 0))
  assert(
    extractions.some((row) => row.code.includes('--')),
    'generated variable keys observed',
  )
  assert(extractions.some((row) => row.updatedBaseStyle || row.cssFile))
  assert.equal(telemetry.events[0], 'reserve')
  if (scenario === 'hoist') checkHoist({ root, mode, single, http, telemetry })
  if (scenario === 'hoist-lazy')
    checkHoist({ root, mode, single, http, telemetry, lazy: true })
  if (scenario === 'environments' || scenario === 'serve-ssr') {
    const rows = reservations[0]?.rows.map((row) => row[0])
    assert(rows)
    for (const name of ['physical/A.tsx', 'physical/B.tsx', 'server.tsx'])
      assert(rows.includes(resolve(root, name).replaceAll('\\', '/')))
  }
  assert(!css.includes('window.never'))
  assert(!css.includes('sentinel-unreachable'))
  assert(!css.includes('sentinel-stale'))
  for (const batch of reservations)
    assert(
      batch.rows.every(
        (row) => !row[0].includes('/out/') && !row[0].includes('/df/'),
      ),
    )
  if (
    !['hmr', 'retained', 'history', 'linked', 'error', 'virtual'].includes(
      scenario,
    )
  ) {
    const arrived = extractions.flatMap((row) =>
      basename(row.id).endsWith('A.tsx')
        ? ['A']
        : basename(row.id).endsWith('B.tsx')
          ? ['B']
          : [],
    )
    assert.deepEqual(
      [...new Set(arrived)],
      [...order],
      'imposed native physical arrival order',
    )
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
