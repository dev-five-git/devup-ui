import assert from 'node:assert/strict'
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const workspace = resolve(import.meta.dirname, '../../../..')
const packageRoot = join(workspace, 'packages/next-plugin')
const required = createRequire(join(packageRoot, 'package.json'))
const format = process.argv[2] ?? 'cjs'
const mode = process.argv[3] ?? 'construct'
assert.ok(mode === 'construct' || mode === 'nested')
const { createMdxIsolateOwner } =
  format === 'cjs'
    ? required('./dist/mdx-isolate-owner.cjs')
    : await import(
        pathToFileURL(join(packageRoot, 'dist/mdx-isolate-owner.mjs')).href
      )
const root = mkdtempSync(
  join(
    process.env.DEVUP_MDX_ISOLATE_TEST_DIR ?? tmpdir(),
    'W21ax-plugin-origin-',
  ),
)
let owner
try {
  symlinkSync(
    join(workspace, 'apps/landing/node_modules'),
    join(root, 'node_modules'),
    'junction',
  )
  writeFileSync(join(root, 'package.json'), '{}')
  const installed = createRequire(join(root, 'package.json'))
  assert.equal(installed('next/package.json').version, '16.3.6')
  const loader = installed.resolve('@next/mdx/mdx-js-loader')
  const compiler = createRequire(loader)
  const mdxModule = compiler.resolve('@mdx-js/mdx')
  const diagnosticModule = createRequire(mdxModule).resolve('vfile-message')
  const plugin = join(root, 'plugin.mjs')
  const trace = join(root, 'plugin-state.jsonl')
  const a = join(root, 'a.mdx')
  const b = join(root, 'b.mdx')
  writeFileSync(a, '# valid A')
  writeFileSync(b, '# valid B')
  writeFileSync(trace, '')
  writeFileSync(
    plugin,
    String.raw`
import { appendFileSync } from 'node:fs'
import { isMainThread, threadId } from 'node:worker_threads'
import { VFileMessage } from ${JSON.stringify(pathToFileURL(diagnosticModule).href)}
import { createProcessor } from ${JSON.stringify(pathToFileURL(mdxModule).href)}
let failedA = false
export default function plugin() {
  return function transformer(tree, file) {
    if (file.path === ${JSON.stringify(a)}) {
      failedA = true
      appendFileSync(${JSON.stringify(trace)}, JSON.stringify({ event: 'A-transform', failedA, isMainThread, threadId }) + '\n')
      ${
        mode === 'construct'
          ? `throw new VFileMessage('plugin stage diagnostic', { source: 'mdast-util-mdx-jsx', ruleId: 'end-tag-mismatch', place: { line: 1, column: 1 } })`
          : `createProcessor({ format: 'mdx' }).parse('<a></b>')`
      }
    }
    if (file.path === ${JSON.stringify(b)}) {
      appendFileSync(${JSON.stringify(trace)}, JSON.stringify({ event: 'B-transform', failedA, isMainThread, threadId }) + '\n')
    }
  }
}
`,
  )
  const options = { format: 'mdx', remarkPlugins: [plugin] }
  const before = structuredClone(options)
  const native = installed(loader)
  const canonical = await new Promise((done, reject) =>
    native.call(
      {
        sourceMap: true,
        context: root,
        resourcePath: b,
        getOptions: () => structuredClone(options),
        async: () => (error, source, map) =>
          error
            ? reject(error)
            : done({ filename: b, source: String(source), map }),
      },
      readFileSync(b, 'utf8'),
    ),
  )
  const pipeline = {
    bundler: 'turbo',
    ruleKey: '*.mdx',
    conditions: [{ condition: { path: /\.mdx$/ } }],
    aliases: {},
    loaders: [{ loader, options }],
  }
  owner = createMdxIsolateOwner(root)
  const signal = new AbortController().signal
  await owner.prepare(
    { run: 'origin', filename: a, pipeline, timeoutMs: 5000 },
    signal,
  )
  const aFailure = await owner
    .bind({
      run: 'origin',
      filename: a,
      canonical: { ...canonical, filename: a },
    })
    .then(
      () => undefined,
      (cause) => cause,
    )
  assert.ok(aFailure instanceof Error)
  await owner.prepare(
    { run: 'origin', filename: b, pipeline, timeoutMs: 5000 },
    signal,
  )
  let publications = 0
  let bytesAgree = false
  const bFailure = await owner
    .bind({ run: 'origin', filename: b, canonical })
    .then(
      (output) => {
        publications++
        bytesAgree = output.source === canonical.source
      },
      (cause) => cause,
    )
  const events = readFileSync(trace, 'utf8')
    .trim()
    .split('\n')
    .filter(Boolean)
    .map((line) => JSON.parse(line))
  const workerA = events.filter(
    (event) => event.event === 'A-transform' && !event.isMainThread,
  )
  const workerB = events.filter(
    (event) => event.event === 'B-transform' && !event.isMainThread,
  )
  assert.equal(
    workerA.length,
    1,
    'fixture must reach configured user transform on valid original A',
  )
  assert.equal(workerA[0].failedA, true)
  assert.deepEqual(
    options,
    before,
    'fixture must not mutate compiler JSON options/H',
  )
  console.info(
    JSON.stringify({
      runtime: process.versions,
      format,
      mode,
      diagnosticModule,
      mdxModule,
      events,
      workerBExecutions: workerB.length,
      publications,
      bytesAgree,
      aFailure: aFailure.message,
      bFailure: bFailure?.message ?? null,
    }),
  )
  assert.equal(
    workerB.length,
    0,
    'unknown plugin failure must quarantine before same-worker B native work',
  )
  assert.equal(
    publications,
    0,
    'unknown plugin failure must not authorize a new B binding',
  )
  assert.ok(bFailure instanceof Error)
  assert.match(bFailure.message, /new build or dev-server restart/)
} finally {
  if (owner) await owner.dispose()
  rmSync(root, { recursive: true, force: true })
}
