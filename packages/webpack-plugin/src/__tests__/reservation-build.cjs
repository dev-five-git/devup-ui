const assert = require('node:assert/strict')
const { mkdirSync } = require('node:fs')
const { join } = require('node:path')
const {
  webpack,
} = require('../../../next-plugin/node_modules/next/dist/compiled/webpack/webpack')
const wasm = require('@devup-ui/wasm')
const { state } = require('./reservation-loaders.cjs')
const fixtureSources = require('./reservation-fixture.cjs')
const snapshot = require('./reservation-output.cjs')
console.info = (...args) => console.error(...args)

const [fixture, schedule, mode, cssMode] = process.argv.slice(2)
assert(fixture && ['AB', 'BA'].includes(schedule))
mkdirSync(fixture, { recursive: true })
process.chdir(fixture)
const watch = mode === 'watch'
const singleCss = cssMode === 'single'
const events = []
const reservations = []
const extracts = []
const atomThresholds = []
let compiling = false
let revision = 0
const originalAtomHoist = wasm.setAtomHoist
wasm.setAtomHoist = (threshold) => {
  atomThresholds.push(threshold)
  return originalAtomHoist(threshold)
}
const originalReserve = wasm.reserveClassSlots
wasm.reserveClassSlots = (...args) => {
  assert(!compiling, 'reservation must precede compilation')
  events.push('reserve')
  const beforeCss = wasm.exportSheet()
  originalReserve(...args)
  assert.equal(wasm.exportSheet(), beforeCss, 'scratch sheet restored exactly')
  reservations.push({
    revision,
    rows: args[0].map(([id]) => id),
    classes: JSON.parse(wasm.exportClassMap()),
    files: JSON.parse(wasm.exportFileMap()),
  })
}
const originalExtract = wasm.codeExtract
wasm.codeExtract = (...args) => {
  const beforeClasses = wasm.exportClassMap()
  const beforeFiles = wasm.exportFileMap()
  const result = originalExtract(...args)
  extracts.push({
    id: args[0],
    prewarm: !compiling,
    residual: wasm.getUnreservedClassSlotCount(),
    sameClasses: beforeClasses === wasm.exportClassMap(),
    sameFiles: beforeFiles === wasm.exportFileMap(),
    code: result.code,
    map: result.map,
    dependencies: result.dependencies,
  })
  events.push(compiling ? 'extract' : 'prewarm')
  return result
}
const { DevupUIWebpackPlugin } = require('../../dist/index.cjs')
const { put, sources } = fixtureSources(fixture, mode)
if (!watch) for (const letter of ['B', 'A']) sources(letter)
let entryCalls = 0
let setups = 0
let compilations = 0
const plugin = new DevupUIWebpackPlugin({
  watch,
  singleCss,
  include: ['@slot/ui'],
  ...(mode === 'debug' ? { debug: true, prefix: 'slot' } : {}),
  ...(['atom', 'noop'].includes(mode) ? { atomHoist: 2 } : {}),
})
const compiler = webpack({
  mode: 'production',
  context: fixture,
  cache: false,
  devtool: false,
  entry:
    mode === 'prewarm'
      ? { main: './src/entry.js' }
      : () => {
          entryCalls += 1
          if (mode === 'once') return { main: ['./src/plain.js'] }
          if (mode === 'invalid') return { main: ['./src/Invalid.js'] }
          return {
            main: [
              './src/entry.js',
              ...(watch && revision === 0
                ? []
                : ['./physical/A.js', './physical/B.js']),
            ],
          }
        },
  output: { path: join(fixture, 'out'), filename: '[name].js', clean: true },
  optimization: {
    minimize: false,
    concatenateModules: false,
    usedExports: false,
  },
  experiments: { css: true },
  module: {
    rules: [
      { test: /\.css$/, type: 'css' },
      { test: /\.(js|ts)$/, use: [join(__dirname, 'reservation-observe.cjs')] },
    ],
  },
  plugins: [
    plugin,
    {
      apply(compiler) {
        setups += 1
        put(
          'src/0-Generated.js',
          `import { css } from '@devup-ui/react'; export const generated = css({ outlineColor: 'lime' });`,
        )
        compiler.options.module.rules.push({
          test: /\.(js|ts)$/,
          enforce: 'pre',
          use: [join(__dirname, 'reservation-loaders.cjs')],
        })
        compiler.hooks.watchRun.tap({ name: 'custody', stage: -10000 }, () => {
          compiling = false
        })
        compiler.hooks.thisCompilation.tap('custody', () => {
          compiling = true
          compilations += 1
        })
        compiler.hooks.done.tap('custody', () => {
          compiling = false
        })
      },
    },
  ],
})
assert(compiler)
async function run() {
  state.order = schedule === 'AB' ? ['A.js', 'B.js'] : ['B.js', 'A.js']
  let initial
  let final
  if (watch) {
    await new Promise((resolve, reject) => {
      const watching = compiler.watch(
        { aggregateTimeout: 0, poll: 100, ignored: /[\\/](df|out)[\\/]/ },
        (error, stats) => {
          if (error) {
            watching.close(() => reject(error))
            return
          }
          try {
            assert(stats)
            if (revision === 0) {
              initial = snapshot(stats, fixture, extracts)
              watching.suspend()
              revision = 1
              for (const letter of schedule) sources(letter)
              compiler.modifiedFiles = new Set(
                [...schedule].map((letter) =>
                  join(fixture, `physical/${letter}.js`),
                ),
              )
              watching.invalidate()
              watching.resume()
            } else {
              final = snapshot(stats, fixture, extracts)
              watching.close((error) => (error ? reject(error) : resolve()))
            }
          } catch (error) {
            watching.close(() => reject(error))
          }
        },
      )
    })
  } else {
    final = await new Promise((resolve, reject) =>
      compiler.run((error, stats) => {
        if (error) {
          reject(error)
          return
        }
        try {
          assert(stats)
          resolve(snapshot(stats, fixture, extracts))
        } catch (error) {
          reject(error)
        }
      }),
    )
  }
  await new Promise((resolve, reject) =>
    compiler.close((error) => (error ? reject(error) : resolve())),
  )
  assert.equal(setups, 1)
  assert.equal(
    entryCalls,
    mode === 'prewarm' ? 0 : compilations,
    'only native compilation may call entry',
  )
  if (!['prewarm', 'once'].includes(mode)) {
    assert(state.dispatched.length >= 2)
    for (let index = 0; index < state.dispatched.length; index += 2)
      assert.deepEqual(state.dispatched.slice(index, index + 2), state.order)
  }
  process.stdout.write(
    JSON.stringify({
      initial,
      final,
      reservations,
      extracts,
      events,
      compilations,
      entryCalls,
      atomThresholds,
    }),
  )
}
run().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
