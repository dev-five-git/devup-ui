const assert = require('node:assert/strict')
const { writeFileSync } = require('node:fs')
const { join } = require('node:path')
const { createRsbuild } = require('@rsbuild/core')
const wasm = require('@devup-ui/wasm')
const { put } = require('./reservation-fixture.cjs')

const [root, scenario, receipt] = process.argv.slice(2)
process.chdir(root)
const batches = [],
  outputs = [],
  events = [],
  hoist = []
const reserve = wasm.reserveClassSlots,
  extract = wasm.codeExtract,
  setHoist = wasm.setAtomHoist
wasm.reserveClassSlots = (...args) => {
  const css = wasm.getCss(null, false)
  reserve(...args)
  assert.equal(wasm.getCss(null, false), css)
  events.push('reserve')
  batches.push(args[0])
}
wasm.codeExtract = (...args) => {
  const before = wasm.getUnreservedClassSlotCount()
  const result = extract(...args)
  events.push('extract')
  outputs.push({
    filename: args[0],
    code: result.code,
    map: result.map,
    before,
    after: wasm.getUnreservedClassSlotCount(),
  })
  return result
}
wasm.setAtomHoist = (value) => {
  hoist.push(value)
  setHoist(value)
}
const { DevupUI } = require('../../dist/index.cjs')
put(root, 'devup.json', '{}')
const atom = scenario === 'atom' || scenario === 'noop'
const options = { singleCss: !atom, atomHoist: atom ? 2 : undefined }
const config = {
  mode: 'production',
  plugins: [DevupUI(options)],
  source: { entry: { main: './src/entry.js' } },
  output: { distPath: { root: 'out' }, minify: false, filenameHash: false },
  performance: { printFileSize: false },
  html: { template: false },
  tools: { rspack: { cache: false, optimization: { minimize: false } } },
}
let entries = 0,
  compilations = 0,
  setups = 0
if (atom) {
  put(
    root,
    'src/shared.jsx',
    'import {Box} from "@devup-ui/react"; export const Shared = () => <Box w="27px" color="purple"/>',
  )
  put(
    root,
    'src/entry.js',
    'import {Shared} from "./shared.jsx"; console.log(Shared)',
  )
  if (scenario === 'atom') {
    put(
      root,
      'src/second.js',
      'import {Shared} from "./shared.jsx"; console.log(Shared)',
    )
    config.source.entry.second = './src/second.js'
  }
} else if (scenario === 'multi' || scenario === 'multi-root') {
  put(
    root,
    'outside-a/entry.js',
    'import {css} from "@devup-ui/react"; console.log(css({color:"olive"}))',
  )
  put(
    root,
    'outside-b/entry.js',
    'import {css} from "@devup-ui/react"; console.log(css({color:"coral"}))',
  )
  config.environments = Object.fromEntries(
    ['a', 'b'].map((name) => [
      name,
      {
        source: {
          entry: {
            main:
              scenario === 'multi-root'
                ? `./index-${name}.js`
                : `./outside-${name}/entry.js`,
          },
        },
        output: { distPath: { root: `out/${name}` } },
      },
    ]),
  )
  if (scenario === 'multi-root')
    for (const name of ['a', 'b'])
      put(root, `index-${name}.js`, `import './outside-${name}/entry.js'`)
} else {
  put(
    root,
    'src/entry.js',
    'import {css} from "@devup-ui/react"; console.log(css({color:"teal"}))',
  )
  put(
    root,
    'external/entry.css.ts',
    'import {createVar} from "@vanilla-extract/css"; export const variable = createVar(); console.log(variable)',
  )
  if (scenario === 'static')
    config.source.entry.main = './external/entry.css.ts'
  if (scenario === 'root') {
    put(
      root,
      'index.js',
      'import {css} from "@devup-ui/react"; import {variable} from "./external/entry.css.ts"; console.log(css({color:"red"}),variable,"import {css} from \'@devup-ui/react\'")',
    )
    config.source.entry.main = './index.js'
  }
}
config.plugins.push({
  name: 'native-observer',
  setup(api) {
    setups++
    if (scenario === 'opaque')
      api.transform(
        { test: /[\\/]src[\\/]entry\.js$/, order: 'pre' },
        ({ code }) => {
          entries++
          return `${code}\nimport "../external/entry.css.ts"`
        },
      )
    api.modifyRspackConfig((rspackConfig) => {
      rspackConfig.plugins.push({
        apply(compiler) {
          compiler.hooks.thisCompilation.tap('native-observer', () => {
            compilations++
          })
        },
      })
    })
  },
})

async function main() {
  const rsbuild = await createRsbuild({ cwd: root, config })
  let result
  try {
    result = await rsbuild.build()
    assert(result.stats && !result.stats.hasErrors())
    const assets = Object.fromEntries(
      (result.stats.stats || [result.stats]).flatMap((stats) =>
        stats.compilation
          .getAssets()
          .map((asset) => [
            join(stats.compilation.outputOptions.path, asset.name),
            String(asset.source.source()),
          ]),
      ),
    )
    writeFileSync(
      receipt,
      JSON.stringify({
        batches,
        outputs,
        events,
        hoist,
        css: wasm.getCss(null, false),
        count: wasm.getUnreservedClassSlotCount(),
        fileMap: wasm.exportFileMap(),
        classMap: wasm.exportClassMap(),
        assets,
        entries,
        compilations,
        setups,
      }),
    )
  } finally {
    if (result) await result.close()
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
