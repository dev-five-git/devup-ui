import assert from 'node:assert/strict'
import { readFile, realpath, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const require = createRequire(import.meta.url)
const format = process.argv[2]
const mode = process.argv[3] ?? 'default'
assert(['default', 'native', 'wasm'].includes(mode))
const options = mode === 'default' ? {} : { engine: mode }
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
const utilsPath = createRequire(entry).resolve('@devup-ui/plugin-utils')
const utils = createRequire(entry)('@devup-ui/plugin-utils')
const selected = utils.loadCompiler(options.engine)
const utilsRoot = dirname(dirname(utilsPath))
const utilsManifest = JSON.parse(
  await readFile(join(utilsRoot, 'package.json'), 'utf8'),
)
const esmUtils = await import(
  pathToFileURL(join(utilsRoot, utilsManifest.exports['.'].import)).href
)
assert.equal(esmUtils.loadCompiler(options.engine).getCss, selected.getCss)
const methods = [
  'registerShorthands',
  'reserveClassSlots',
  'codeExtract',
  'getCss',
  'exportSheet',
  'exportClassMap',
  'exportFileMap',
]
const original = Object.fromEntries(
  methods.map((name) => [name, selected[name]]),
)
const calls = Object.fromEntries(methods.map((name) => [name, 0]))
for (const name of methods) {
  selected[name] = (...args) => {
    calls[name]++
    return original[name](...args)
  }
}
const { webpack } = require('next/dist/compiled/webpack/webpack')
const compiler = webpack({
  mode: 'production',
  optimization: { minimize: false },
  entry: './src/main.jsx',
  plugins: [new plugin.DevupUIWebpackPlugin(options)],
})
try {
  const loaders = compiler.options.module.rules
    .flatMap((rule) => rule.use || [])
    .map((use) => use.loader)
    .filter(Boolean)
  const selectedLoaders = await Promise.all(
    loaders.map((path) => realpath(path)),
  )
  const utilsPaths = await Promise.all(
    [
      utilsPath,
      ...loaders.map((path) =>
        createRequire(path).resolve('@devup-ui/plugin-utils'),
      ),
    ].map((path) => realpath(path)),
  )
  assert.deepEqual(utilsPaths, [utilsPaths[0], utilsPaths[0], utilsPaths[0]])
  for (const path of loaders) {
    const fromLoader = createRequire(path)(
      '@devup-ui/plugin-utils',
    ).loadCompiler(options.engine)
    for (const name of methods) {
      assert.equal(fromLoader[name], selected[name])
    }
  }
  const uses = compiler.options.module.rules.flatMap((rule) => rule.use || [])
  for (const use of uses) {
    assert.equal(use.options.engine, options.engine ?? 'native')
  }
  const reservation = compiler.hooks.run.taps.find(
    ({ name }) => name === 'DevupUIWebpackPluginReservation',
  )
  assert(reservation)
  reservation.fn(compiler)
  const sourceModule =
    format === 'import'
      ? (await import('@devup-ui/webpack-plugin/loader')).default
      : require('@devup-ui/webpack-plugin/loader')
  const cssModule =
    format === 'import'
      ? (await import('@devup-ui/webpack-plugin/css-loader')).default
      : require('@devup-ui/webpack-plugin/css-loader')
  const sourceLoader = sourceModule.default ?? sourceModule
  const cssLoader = cssModule.default ?? cssModule
  const sourcePath = join(process.cwd(), 'src/Fixture.jsx')
  const source = await readFile(sourcePath)
  await new Promise((resolve, reject) => {
    sourceLoader.call(
      {
        resourcePath: sourcePath,
        getOptions: () => ({ ...uses[0].options, watch: true }),
        addDependency() {},
        async: () => (error, code) => {
          if (error) {
            reject(error)
          } else {
            assert.equal(typeof code, 'string')
            resolve()
          }
        },
      },
      source,
    )
  })
  cssLoader.call(
    {
      resourcePath: join(uses[0].options.cssDir, 'devup-ui.css'),
      getOptions: () => uses[1].options,
      cacheable(value) {
        assert.equal(value, false)
      },
      callback(error, css) {
        assert.equal(error, null)
        assert.equal(typeof css, 'string')
      },
    },
    Buffer.from(''),
  )
  for (const name of methods) {
    assert(calls[name] > 0, `selected namespace must observe ${name}`)
  }
  const evidence = {
    format,
    entry: await realpath(entry),
    expectedEntry,
    packageRoot,
    selectedLoaders,
    expectedLoaders,
    mode,
    utilsPaths,
    calls,
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
} finally {
  for (const name of methods) {
    selected[name] = original[name]
  }
  await new Promise((resolve, reject) =>
    compiler.close((error) => (error ? reject(error) : resolve())),
  )
}
