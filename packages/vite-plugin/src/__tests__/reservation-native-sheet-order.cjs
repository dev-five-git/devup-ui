const assert = require('node:assert/strict')
const { writeFileSync } = require('node:fs')
const { basename, resolve } = require('node:path')
const { put } = require('./reservation-native-fixture.cjs')
const { text } = require('./reservation-native-receipt.cjs')

const args = process.argv.slice(2)
const root = text(args[0]),
  order = text(args[1]),
  receipt = text(args[2])
assert(order === 'sheet-first' || order === 'member-first')
process.chdir(root)
put(root, 'devup.json', '{}')
put(root, 'package.json', '{"type":"module"}')
// A.tsx has a single importer, so its atoms land in the sheet of root.js.
put(
  root,
  'root.js',
  "import './A.tsx'; import {css} from '@devup-ui/react'; console.log(css({outlineColor:'purple'}))",
)
put(
  root,
  'A.tsx',
  "import {css} from '@devup-ui/react'; console.log(css({color:'red'}))",
)
/** @type {{DevupUI:(options:{engine:'wasm',distDir:string,cssDir:string,devupFile:string,singleCss:boolean})=>import('vite').PluginOption}} */
const { DevupUI } = require('../../dist/' + 'index.cjs')

/** @param {string} id */
const isSheet = (id) =>
  /^devup-ui(?:-\d+)?\.css$/.test(basename(id).split('?')[0] ?? '')
/** @type {string[]} */
const events = []
/** @type {Set<string>} */
const loadedSheets = new Set()
// root.js imports two sheets: the base sheet and its own bucket sheet.
const sheetsLoaded = Promise.withResolvers()
const memberExtracted = Promise.withResolvers()
/** @param {import('rolldown').OutputBundle} bundle */
function contents(bundle) {
  /** @type {Record<string, string>} */
  const result = {}
  for (const item of Object.values(bundle))
    result[item.fileName] =
      item.type === 'chunk' ? item.code : String(item.source)
  return result
}
/** @type {Record<string, string>} */
let snapshot = {}
/** @type {Record<string, string>} */
let assets = {}

async function main() {
  const { build } = await import('vite')
  await build({
    configFile: false,
    root,
    logLevel: 'error',
    plugins: [
      {
        name: 'force-sheet-order',
        enforce: 'pre',
        async load(id) {
          // Vite snapshots a sheet's content when it is loaded.
          if (order === 'member-first' && isSheet(id))
            await memberExtracted.promise
        },
        async transform(_code, id) {
          if (isSheet(id)) {
            events.push(`load:${basename(id).split('?')[0]}`)
            loadedSheets.add(id)
            if (loadedSheets.size === 2) sheetsLoaded.resolve(undefined)
          } else if (order === 'sheet-first' && basename(id) === 'A.tsx')
            await sheetsLoaded.promise
        },
        generateBundle(_options, bundle) {
          // Runs before devup-ui finalizes its sheets.
          snapshot = contents(bundle)
        },
      },
      DevupUI({
        engine: 'wasm',
        distDir: resolve(root, 'df'),
        cssDir: resolve(root, 'df/devup-ui'),
        devupFile: resolve(root, 'devup.json'),
        singleCss: false,
      }),
      {
        name: 'observe-extraction',
        transform(_code, id) {
          if (basename(id) !== 'A.tsx') return
          events.push('extract:A.tsx')
          memberExtracted.resolve(undefined)
        },
        writeBundle(_options, bundle) {
          assets = contents(bundle)
        },
      },
    ],
    build: {
      minify: false,
      outDir: 'out',
      rolldownOptions: {
        input: 'root.js',
        output: {
          entryFileNames: '[name].js',
          chunkFileNames: '[name].js',
          assetFileNames: '[name][extname]',
        },
      },
    },
    optimizeDeps: { noDiscovery: true, include: [] },
  })
  writeFileSync(receipt, JSON.stringify({ events, snapshot, assets }, null, 2))
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
