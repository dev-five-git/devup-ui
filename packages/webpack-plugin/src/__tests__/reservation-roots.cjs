const assert = require('node:assert/strict')
const { join } = require('node:path')
const {
  webpack,
} = require('../../../next-plugin/node_modules/next/dist/compiled/webpack/webpack')
const wasm = require('@devup-ui/wasm')
const { state } = require('./reservation-loaders.cjs')
const snapshot = require('./reservation-output.cjs')

const [fixture, mode = 'source'] = process.argv.slice(2)
assert(fixture)
process.chdir(fixture)
const reservations = []
const extracts = []
const events = []
let compiling = false
let compilations = 0
let entryCalls = 0
const originalReserve = wasm.reserveClassSlots
wasm.reserveClassSlots = (...args) => {
  assert(!compiling)
  originalReserve(...args)
  reservations.push({ revision: 0, rows: args[0].map(([id]) => id) })
  events.push('reserve')
}
const originalExtract = wasm.codeExtract
wasm.codeExtract = (...args) => {
  const classes = wasm.exportClassMap()
  const files = wasm.exportFileMap()
  const output = originalExtract(...args)
  state.transformed[args[0]] = output.code
  extracts.push({
    id: args[0],
    prewarm: !compiling,
    sameClasses: classes === wasm.exportClassMap(),
    sameFiles: files === wasm.exportFileMap(),
    residual: wasm.getUnreservedClassSlotCount(),
    map: output.map,
    dependencies: output.dependencies,
  })
  events.push(compiling ? 'extract' : 'prewarm')
  return output
}
const { DevupUIWebpackPlugin } = require('../../dist/index.cjs')
const compiler = webpack({
  mode: 'production',
  context: mode === 'context-app' ? join(fixture, 'project') : fixture,
  entry: ['opaque', 'app', 'context-app'].includes(mode)
    ? () => {
        entryCalls += 1
        return {
          main:
            mode === 'opaque' ? './external/entry.css.ts' : './app/entry.js',
        }
      }
    : {
        main:
          mode === 'root'
            ? './index.js'
            : mode === 'static'
              ? './external/entry.js'
              : './src/entry.js',
      },
  cache: false,
  devtool: false,
  output: { path: join(fixture, 'out'), filename: '[name].js' },
  optimization: {
    minimize: false,
    concatenateModules: false,
    usedExports: false,
  },
  experiments: { css: true },
  module: { rules: [{ test: /\.css$/, type: 'css' }] },
  plugins: [new DevupUIWebpackPlugin({ atomHoist: 0 })],
})
assert(compiler)
compiler.hooks.thisCompilation.tap('root-custody', () => {
  compiling = true
  compilations += 1
})
async function run() {
  try {
    const final = await new Promise((resolve, reject) => {
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
      })
    })
    process.stdout.write(
      JSON.stringify({
        final,
        reservations,
        extracts,
        events,
        compilations,
        entryCalls,
        atomThresholds: [],
      }),
    )
  } finally {
    await new Promise((resolve, reject) =>
      compiler.close((error) => (error ? reject(error) : resolve())),
    )
  }
}
run().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
