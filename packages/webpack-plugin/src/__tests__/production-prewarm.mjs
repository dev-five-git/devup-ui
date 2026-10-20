import assert from 'node:assert/strict'
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, relative } from 'node:path'

import { codeExtract, exportFileMap, getCss } from '@devup-ui/wasm'

import { DevupUI } from '../../../next-plugin/src/plugin.ts'
import { setWebpackPluginForTesting } from '../../../next-plugin/src/wasm.ts'
import * as webpackPlugin from '../index.ts'
import { DevupUIWebpackPlugin } from '../plugin.ts'

const originalCwd = process.cwd()
const fromNextPlugin = createRequire(
  new URL('../../../next-plugin/package.json', import.meta.url),
)
const { webpack } = fromNextPlugin('next/dist/compiled/webpack/webpack')
const root = mkdtempSync(join(tmpdir(), 'devup-webpack-prewarm-'))
const compilers = new Set()
const failures = []
function write(file, source) {
  mkdirSync(dirname(join(root, file)), { recursive: true })
  writeFileSync(join(root, file), source)
}
function apply(options = {}, entry = 'next-app-loader?query!') {
  const compiler = webpack({
    mode: 'production',
    context: root,
    entry,
    optimization: { minimizer: [] },
    plugins: [new DevupUIWebpackPlugin(options)],
  })
  compilers.add(compiler)
  return compiler
}
async function prepare(compiler) {
  // Unit boundary only: this synthetic Next request is not a runnable module graph.
  // Invoke only our registered run preparation, which reserves scratch before extraction.
  const taps = compiler.hooks.run.taps.filter(
    ({ name }) => name === 'DevupUIWebpackPluginReservation',
  )
  assert.equal(taps.length, 1)
  await taps[0].fn(compiler)
}
async function close(compiler) {
  try {
    await new Promise((resolve, reject) =>
      compiler.close((error) => (error ? reject(error) : resolve())),
    )
  } finally {
    compilers.delete(compiler)
  }
}
function extract(file, singleCss) {
  return codeExtract(
    file,
    readFileSync(join(root, file), 'utf8'),
    '@devup-ui/react',
    `./${relative(dirname(join(root, file)), join(root, 'df/devup-ui')).replaceAll('\\', '/')}`,
    singleCss,
    false,
    true,
    {},
  ).code
}
function sheet() {
  return [
    getCss(null, false),
    ...Object.values(JSON.parse(exportFileMap())).map((number) =>
      getCss(number, false),
    ),
  ].join('\n')
}
try {
  process.chdir(root)
  write('package.json', '{"private":true}')
  write('devup.json', '{}')
  write(
    'src/page.tsx',
    'import { Box } from "@devup-ui/react"; export const page = <Box color="maroon"/>',
  )
  write(
    'src/unreachable.tsx',
    'import { Box } from "@devup-ui/react"; export const unused = <Box color="goldenrod"/>',
  )
  write(
    'node_modules/design-system/child.tsx',
    'import { Box } from "@devup-ui/react"; export const child = <Box color="cyan"/>',
  )
  write('node_modules/design-system/package.json', '{"main":"index.js"}')
  write('node_modules/design-system/index.js', 'export * from "./child"')
  write('src/barrel.ts', 'export * from "design-system"')
  if (process.env.DEVUP_PREWARM_CLEANUP_PROOF === '1') {
    // Given outstanding compilers whose asynchronous shutdowns both fail.
    for (const name of ['first', 'second']) {
      const compiler = apply({ productionPrewarmFiles: [] })
      compiler.hooks.shutdown.tapAsync('cleanup-proof', (callback) => {
        setImmediate(() => {
          assert.equal(existsSync(root), true)
          console.info(`shutdown:${name}`)
          callback(new Error(`cleanup:${name}`))
        })
      })
    }
    // When an assertion aborts the proof, finally must await both and preserve every error.
    assert.fail('initialization-proof-failure')
  }
  // Given production Next callbacks, discovery supplies the same complete set without extracting.
  setWebpackPluginForTesting(webpackPlugin)
  const next = DevupUI({}, { include: ['design-system'] })
  const publicConfig = {}
  const configured = ['server', 'edge-server', 'client'].map((name) => {
    const config = next.webpack(
      { plugins: [] },
      {
        buildId: 'proof',
        config: publicConfig,
        dev: false,
        isServer: name !== 'client',
        name,
      },
    )
    return config.plugins[0].options.productionPrewarmFiles
  })
  assert.deepEqual(configured[0], [
    'node_modules/design-system/child.tsx',
    'node_modules/design-system/index.js',
    'src/barrel.ts',
    'src/page.tsx',
    'src/unreachable.tsx',
  ])
  assert.deepEqual(configured[1], configured[0])
  assert.deepEqual(configured[2], configured[0])
  assert.doesNotMatch(sheet(), /maroon|cyan|goldenrod/)
  for (const singleCss of [false, true]) {
    // Given the complete set, including an external file hidden by a Next query.
    const productionPrewarmFiles = [
      'src/page.tsx',
      'node_modules/design-system/child.tsx',
    ]
    let compiler = apply({ productionPrewarmFiles, singleCss })
    assert.doesNotMatch(sheet(), /maroon|cyan|goldenrod/)
    await prepare(compiler)
    const css = sheet()
    assert.match(css, /color:maroon/)
    assert.match(css, /color:cyan/)
    assert.doesNotMatch(css, /goldenrod/)
    const code = productionPrewarmFiles.map((file) => extract(file, singleCss))
    await close(compiler)
    // When the same engine is reset by another compiler with reordered duplicate paths.
    compiler = apply({
      singleCss,
      productionPrewarmFiles: [
        join(root, 'node_modules/design-system/child.tsx'),
        './src/../src/page.tsx',
        'src/page.tsx',
        'src\\page.tsx',
      ],
    })
    await prepare(compiler)
    // Then both CSS bytes and transformed class correspondence are identical.
    assert.equal(sheet(), css)
    assert.deepEqual(
      productionPrewarmFiles.map((file) => extract(file, singleCss)),
      code,
    )
    await close(compiler)
  }
  // Given a generic webpack entry, unreachable source remains excluded.
  let compiler = apply({}, './src/page.tsx')
  await prepare(compiler)
  assert.match(sheet(), /maroon/)
  assert.doesNotMatch(sheet(), /cyan|goldenrod/)
  await close(compiler)
  // When the complete set is explicitly empty, then even that entry is not prewarmed.
  compiler = apply({ productionPrewarmFiles: [] }, './src/page.tsx')
  await prepare(compiler)
  assert.doesNotMatch(sheet(), /maroon|cyan|goldenrod/)
  await close(compiler)
  compiler = apply({ watch: true, productionPrewarmFiles: ['src/page.tsx'] })
  await prepare(compiler)
  assert.doesNotMatch(sheet(), /maroon/)
  await close(compiler)
  compiler = apply({ productionPrewarmFiles: ['missing.tsx'] })
  await assert.rejects(prepare(compiler))
  await close(compiler)
  write(
    'bad.tsx',
    'import { Box } from "@devup-ui/react"; export const c = <Box color={}/>',
  )
  compiler = apply({ productionPrewarmFiles: ['bad.tsx'] })
  await assert.rejects(prepare(compiler))
  await close(compiler)
  for (const invalid of ['missing.tsx', 'bad.tsx']) {
    // Given a valid file sorted before the failing file, initialization partially extracts.
    write(
      'a-valid.tsx',
      'import { Box } from "@devup-ui/react"; export const c = <Box color="chartreuse" insetX="19px"/>',
    )
    // When run preparation rejects, the returned compiler still needs awaited shutdown.
    compiler = apply({
      prefix: 'failed',
      shorthands: { insetX: ['left', 'right'] },
      atomHoist: 2,
      productionPrewarmFiles: ['a-valid.tsx', invalid],
    })
    await assert.rejects(prepare(compiler))
    assert.match(sheet(), /chartreuse/)
    assert.match(sheet(), /left:19px/)
    await close(compiler)
    // Then an unrelated generic build resets the partial styles and engine options.
    compiler = apply({}, './src/page.tsx')
    await prepare(compiler)
    assert.match(sheet(), /maroon/)
    assert.doesNotMatch(sheet(), /chartreuse|failed|left:19px/)
    const cleanCode = codeExtract(
      'clean.tsx',
      'import { Box } from "@devup-ui/react"; export const c = <Box insetX="23px"/>',
      '@devup-ui/react',
      './df/devup-ui',
      false,
      false,
      true,
      {},
    ).code
    assert.doesNotMatch(cleanCode, /failed/)
    assert.match(sheet(), /inset-x:23px/)
    assert.doesNotMatch(sheet(), /left:23px/)
    await close(compiler)
  }
  console.info('production prewarm proof passed')
} catch (error) {
  failures.push(error)
} finally {
  const results = await Promise.allSettled([...compilers].map(close))
  for (const result of results) {
    if (result.status === 'rejected') failures.push(result.reason)
  }
  setWebpackPluginForTesting(undefined)
  process.chdir(originalCwd)
  try {
    rmSync(root, { recursive: true, force: true })
    if (process.env.DEVUP_PREWARM_CLEANUP_PROOF === '1')
      console.info('fixture-removed')
  } catch (error) {
    failures.push(error)
  }
}
if (failures.length === 1) throw failures[0]
if (failures.length > 1)
  throw new AggregateError(failures, 'Prewarm proof and cleanup failed')
