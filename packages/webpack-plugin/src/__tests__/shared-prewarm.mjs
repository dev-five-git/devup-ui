import assert from 'node:assert/strict'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  getCss,
} from '@devup-ui/wasm'

import { compilerScope, createWebpackGeneration } from '../build-scope.ts'
import { DevupUIWebpackPlugin } from '../plugin.ts'

const installed = createRequire(
  new URL('../../../../apps/landing/package.json', import.meta.url),
)
const { webpack } = installed('next/dist/compiled/webpack/webpack')
const root = mkdtempSync(join(tmpdir(), 'devup-shared-prewarm-'))
const original = process.cwd()
const compilers = []
function write(file, source) {
  mkdirSync(dirname(join(root, file)), { recursive: true })
  writeFileSync(join(root, file), source)
}
function configure(entry, binding) {
  const compiler = webpack({
    context: root,
    mode: 'production',
    entry,
    optimization: { minimize: false },
    plugins: [
      new DevupUIWebpackPlugin(
        { productionPrewarmFiles: ['src/page.tsx', 'src/other.tsx'] },
        binding,
      ),
    ],
  })
  compilers.push(compiler)
  return compiler
}
async function prepare(compiler) {
  await compiler.hooks.beforeRun.promise(compiler)
  const taps = compiler.hooks.run.taps.filter(
    ({ name }) => name === 'DevupUIWebpackPluginReservation',
  )
  assert.equal(taps.length, 1)
  await taps[0].fn(compiler)
}
function replay(compiler) {
  const scope = compilerScope(compiler)
  assert.ok(scope)
  return scope.run(() => {
    const output = codeExtract(
      'src/page.tsx',
      readFileSync(join(root, 'src/page.tsx'), 'utf8'),
      '@devup-ui/react',
      '../df/devup-ui',
      false,
      false,
      true,
      {},
    )
    try {
      const files = JSON.parse(exportFileMap())
      return {
        code: output.code,
        classes: JSON.parse(exportClassMap()),
        files,
        css: [
          getCss(null, false),
          ...Object.values(files).map((number) => getCss(number, false)),
        ].join('\n'),
      }
    } finally {
      output.free()
    }
  })
}
try {
  process.chdir(root)
  // Given: server fallback routes and two physical excluded client entries share one complete inventory.
  write('package.json', '{"private":true}')
  write(
    'src/page.tsx',
    'import {Box} from "@devup-ui/react";export function Page({tone}){return <Box bg="maroon" color={tone}/>}',
  )
  write('src/other.tsx', 'export const other = 1')
  const entries = ['next.js', 'app-next.js'].map((file) =>
    join(root, 'node_modules/next/dist/client', file),
  )
  for (const file of entries)
    write(file.slice(root.length + 1), 'export const framework = 1')
  const owner = createWebpackGeneration()
  const server = configure('next-app-loader?query!', { owner, complete: false })
  await prepare(server)
  const before = replay(server)
  assert.match(before.css, /background:maroon/)
  assert.match(before.css, /color:var\(--[\w-]+\)/)
  // When: the sibling config prewarms and re-extracts exactly the same source through its own configuration.
  const client = configure(entries, { owner, complete: true })
  await prepare(client)
  const after = replay(client)
  console.info(JSON.stringify({ before, after }))
  if (process.env.DEVUP_CSS_EVIDENCE_DIR)
    writeFileSync(
      join(process.env.DEVUP_CSS_EVIDENCE_DIR, 'shared-prewarm.json'),
      JSON.stringify({ before, after }, null, 2),
    )
  // Then: selected classes/variables, allocator identities and emitted declarations remain unchanged.
  assert.equal(after.code, before.code)
  assert.deepEqual(after.classes, before.classes)
  assert.deepEqual(after.files, before.files)
  assert.equal(after.css, before.css)
} finally {
  try {
    for (const compiler of compilers)
      await new Promise((done, reject) =>
        compiler.close((error) => (error ? reject(error) : done())),
      )
  } finally {
    process.chdir(original)
    rmSync(root, { recursive: true, force: true })
  }
}
