import assert from 'node:assert/strict'
import { readFile, realpath, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const require = createRequire(import.meta.url)
const format = process.argv[2]
let plugin
let entry
switch (format) {
  case 'require':
    plugin = require('@devup-ui/webpack-plugin')
    entry = require.resolve('@devup-ui/webpack-plugin')
    break
  case 'import':
    plugin = await import('@devup-ui/webpack-plugin')
    entry = fileURLToPath(import.meta.resolve('@devup-ui/webpack-plugin'))
    break
  default:
    throw new Error(`Unknown public entry format ${format}`)
}

const packageRoot = await realpath(
  join(import.meta.dirname, 'node_modules/@devup-ui/webpack-plugin'),
)
const manifest = JSON.parse(
  await readFile(join(packageRoot, 'package.json'), 'utf8'),
)
const expectedEntry = await realpath(
  join(packageRoot, manifest.exports['.'][format]),
)
const expectedLoaders = await Promise.all(
  ['./loader', './css-loader'].map((name) =>
    realpath(join(packageRoot, manifest.exports[name].require)),
  ),
)
const { webpack } = require('next/dist/compiled/webpack/webpack')
const compiler = webpack({
  mode: 'production',
  optimization: { minimize: false },
  entry: './src/main.jsx',
  plugins: [new plugin.DevupUIWebpackPlugin()],
})
try {
  const loaders = compiler.options.module.rules
    .flatMap((rule) => rule.use || [])
    .map((use) => use.loader)
    .filter(Boolean)
  const selectedLoaders = await Promise.all(
    loaders.map((path) => realpath(path)),
  )
  const entryWasmPath =
    format === 'import'
      ? fileURLToPath(
          import.meta.resolve('@devup-ui/wasm', pathToFileURL(entry).href),
        )
      : createRequire(entry).resolve('@devup-ui/wasm')
  const wasmPaths = await Promise.all(
    [
      entryWasmPath,
      ...loaders.map((path) => createRequire(path).resolve('@devup-ui/wasm')),
    ].map((path) => realpath(path)),
  )
  const evidence = {
    format,
    entry: await realpath(entry),
    expectedEntry,
    packageRoot,
    selectedLoaders,
    expectedLoaders,
    wasmPaths,
  }
  await writeFile(
    join(import.meta.dirname, `identity-${format}.json`),
    JSON.stringify(evidence, null, 2),
  )
  console.info(JSON.stringify(evidence))
  assert.equal(evidence.entry, expectedEntry)
  assert.deepEqual(
    selectedLoaders,
    expectedLoaders,
    'both loaders must belong to the installed plugin',
  )
  const wasmRoot = await realpath(
    join(import.meta.dirname, 'node_modules/@devup-ui/wasm'),
  )
  const wasmManifest = JSON.parse(
    await readFile(join(wasmRoot, 'package.json'), 'utf8'),
  )
  const consumerWasm = await realpath(
    join(wasmRoot, wasmManifest.exports['.'][format]),
  )
  assert.deepEqual(
    wasmPaths,
    [consumerWasm, consumerWasm, consumerWasm],
    'entry and both loaders must share the consumer WASM file',
  )
  const entryWasm = await import(pathToFileURL(entryWasmPath).href)
  for (const path of loaders) {
    assert.equal(createRequire(path)('@devup-ui/wasm').getCss, entryWasm.getCss)
  }
} finally {
  await new Promise((resolve, reject) =>
    compiler.close((error) => (error ? reject(error) : resolve())),
  )
}
