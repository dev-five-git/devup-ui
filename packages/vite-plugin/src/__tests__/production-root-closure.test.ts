import { realpathSync } from 'node:fs'
import { symlink } from 'node:fs/promises'
import { resolve } from 'node:path'

import {
  mergeImportAliases,
  type ProductionManifestContext,
} from '@devup-ui/plugin-utils'
import { afterEach, expect, it } from 'bun:test'
import { resolveConfig } from 'vite'

import {
  createExtractionGate,
  prepareProductionManifest,
  type ProductionOptions,
} from '../production-manifest'
import { productionRootClosure } from '../production-root-closure'
import { productionRootSources } from '../production-root-sources'
import { productionFixture } from './production-fixture'

const options: ProductionOptions = {
  package: '@devup-ui/react',
  devupFile: 'devup.json',
  distDir: 'df',
  cssDir: undefined,
  extractCss: true,
  debug: false,
  include: ['@acme/ui', 'included'],
  singleCss: false,
  prefix: undefined,
  shorthands: undefined,
  sourceDirs: undefined,
  mdxExtensions: ['.mdx', '.custom'],
  atomHoist: undefined,
  importAliases: {},
}
const gate = createExtractionGate(options)
const needle = '/* @devup-ui/react */'
const posix = (path: string) => path.replaceAll('\\', '/')
const cleanups: (() => Promise<void>)[] = []
async function fixture() {
  const f = await productionFixture()
  cleanups.push(f.close)
  return f
}
afterEach(async () => {
  for (const close of cleanups.splice(0)) await close()
})
function context(
  root: string,
  paths: readonly string[],
  observed: string[] = [],
): ProductionManifestContext {
  return {
    key: 'test',
    files: paths.map((path) => ({ path, realPath: realpathSync.native(path) })),
    resolverOptions: {
      cwd: root,
      includeMdx: options.mdxExtensions,
      conditions: ['browser'],
      alias: { ignored: false },
      onResolutionInputs: (inputs) => observed.push(...inputs.fileDependencies),
    },
    toId: posix,
  }
}

it('keeps allowed package conditions and denies foreign/output poison before metadata IO', async () => {
  // Given both Devup scopes, explicit includes and malformed foreign/output metadata.
  const f = await fixture()
  const allowed = ['@devup-ui/ui', '@devup-editor/ui', '@acme/ui', 'included']
  const foreign = ['foreign', '@foreign/ui']
  const imports = [...allowed, ...foreign, 'ignored', '../output']
  const code = imports.map((name) => `import '${name}';`).join('\n')
  const entry = await f.file('app/src/unused.ts', code + needle)
  const expected = [entry]
  for (const name of allowed) {
    const base = `app/node_modules/${name}`
    await f.file(
      `${base}/package.json`,
      '{"exports":{"browser":"./dist/b.js","default":"./dist/n.js"}}',
    )
    expected.push(await f.file(`${base}/dist/b.js`, needle))
    await f.file(`${base}/dist/n.js`, needle)
  }
  for (const name of foreign)
    await f.file(`app/node_modules/${name}/package.json`, '{broken')
  await f.file('app/output/package.json', '{broken')
  const observed: string[] = []
  // When configured absolute outputs take priority over package eligibility.
  const files = productionRootClosure(
    context(f.root, [entry], observed),
    { ...options, exclude: [`${f.root}/output`] },
    gate,
  )
  // Then every positive survives without observing any forbidden metadata.
  expect(files.map((file) => file.id).sort()).toEqual(expected.sort())
  for (const name of foreign)
    expect(observed.map(posix)).not.toContain(
      `${f.root}/node_modules/${name}/package.json`,
    )
  expect(observed.map(posix)).not.toContain(`${f.root}/output/package.json`)
})

it('follows local aliases and census-excluded imports while reserving raw Markdown without prose scanning', async () => {
  // Given selected custom/test basenames, relative constants and raw Markdown.
  const f = await fixture()
  await f.file(
    'app/tsconfig.json',
    '{"compilerOptions":{"baseUrl":".","paths":{"@local/*":["src/*"]}}}',
  )
  const imports =
    "import '@local/fixtures/c'; import './dist/a.test.js'; import './r.custom'; import './plain.js'; import './types.d.ts'; import './u.json';"
  const entry = await f.file('app/src/root.tsx', imports + needle)
  const c = await f.file('app/src/fixtures/c.ts', needle)
  const test = await f.file('app/src/dist/a.test.js', needle)
  const mdx = await f.file('app/src/r.custom', "import 'foreign';\n# No needle")
  const relevant = await f.file('app/src/needle.mdx', needle)
  await f.file('app/src/plain.mdx', '# No needle')
  await f.file('app/src/plain.js', 'export const plain=1;')
  await f.file('app/src/u.json', '{}')
  const negatives =
    'noise.test.mdx noise.spec.custom noise.test.ts types.d.ts'.split(' ')
  for (const name of negatives) await f.file(`app/src/${name}`, needle)
  await f.file('app/node_modules/foreign/package.json', '{broken')
  const seeds = productionRootSources(
    { roots: [`${f.root}/src`], includeMdx: options.mdxExtensions },
    [],
  )
  // When census filters apply only to seeds, not reached local sources.
  const files = productionRootClosure(
    { ...context(f.root, []), files: seeds },
    options,
    gate,
  )
  // Then only relevant or explicitly reached selected sources reserve IDs.
  expect(files.map((file) => file.id).sort()).toEqual(
    [entry, c, test, mdx, relevant].sort(),
  )
})

it('retains sibling lexical identities and cuts physical cycles in opposite orders', async () => {
  // Given two aliases and a backedge to their physical parent.
  const f = await fixture()
  const source = await f.file(
    'app/shared/a.js',
    "import './loop/a.js';" + needle,
  )
  for (const path of ['shared/loop', 'left', 'right'])
    await symlink(`${f.root}/shared`, `${f.root}/${path}`, 'junction')
  const seeds = [`${f.root}/left/a.js`, `${f.root}/right/a.js`, source]
  // When the iterative queue sees both arrival orders.
  const results = [seeds, seeds.toReversed()].map((paths) =>
    productionRootClosure(context(f.root, paths), {}, gate)
      .map((file) => file.id)
      .sort(),
  )
  // Then cycles terminate without collapsing lexical sibling IDs.
  expect(results[0]).toEqual(results[1])
  for (const seed of seeds) expect(results[0]).toContain(seed)
  expect(results[0]).toHaveLength(6)
})

it.each([false, true])(
  'reopens external cycle frontiers when descendant=%s',
  async (descendant) => {
    // Given two censused roots, an external alias and an independently reached bridge.
    const f = await fixture()
    const file = (path: string, code = '') =>
      f.file(`app/${path}`, code + needle)
    const edge = descendant ? '../bridge.js' : '../link/a.js'
    const a = await file(
      'ext/real/a.js',
      `import '${edge}'; import './leaf.js';`,
    )
    const leaf = await file('ext/real/leaf.js')
    const bridge = await file('ext/bridge.js', "import './link/a.js';")
    const relay = await file('ext/relay.js', "import './bridge.js';")
    const first = await file('src/a.js', "import '../ext/real/a.js';")
    const last = await file(
      'src/z.js',
      `import '../ext/${descendant ? 'relay' : 'bridge'}.js';`,
    )
    await symlink(`${f.root}/ext/real`, `${f.root}/ext/link`, 'junction')
    const alias = `${f.root}/ext/link/a.js`
    const aliasLeaf = `${f.root}/ext/link/leaf.js`
    const expected = [first, last, a, leaf, bridge, alias, aliasLeaf]
      .concat(descendant ? [relay] : [])
      .sort()
    const results: string[][] = []
    // When opposite root orders reach an unexpanded cut or an expanded, descendant-cut bridge.
    for (const roots of [
      [first, last],
      [last, first],
    ]) {
      const files = productionRootClosure(context(f.root, roots), {}, gate)
      const ids = files.map((file) => file.id).sort()
      // Then BOTH lexical leaves survive, not merely equal but incomplete memberships.
      expect(ids).toContain(leaf)
      expect(ids).toContain(aliasLeaf)
      expect(ids).toEqual(expected)
      results.push(ids)
    }
    expect(results[0]).toEqual(results[1])
  },
)

it('reports real source IO failure at the filename with its original cause', async () => {
  // Given an admitted but missing source.
  const f = await fixture()
  const path = `${f.root}/missing.js`
  const input = { ...context(f.root, []), files: [{ path, realPath: path }] }
  // When the real source read fails.
  let failure: unknown
  try {
    productionRootClosure(input, {}, gate)
  } catch (cause) {
    failure = cause
  }
  // Then the located fatal error retains the filesystem failure.
  if (!(failure instanceof Error)) throw new TypeError('Expected source error')
  expect(failure.message).toContain(`${path}:1:1:`)
  expect(failure.cause).toHaveProperty('code', 'ENOENT')
})

it('prepares actual landing reservations within a bounded source-only deadline', async () => {
  // Given checkout sources/products, not a copied count oracle.
  const root = resolve(import.meta.dir, '../../../../apps/landing')
  const started = performance.now()
  const config = await resolveConfig({ root, configLoader: 'native' }, 'build')
  const observed: string[] = []
  // When production prepares every actual context.
  const manifest = await prepareProductionManifest(
    config,
    {
      ...options,
      include: [],
      mdxExtensions: ['.mdx'],
      importAliases: mergeImportAliases(),
    },
    (inputs) => observed.push(...inputs.fileDependencies),
  )
  // Then Title/components/reset survive without foreign traversal.
  for (const context of manifest.contexts) {
    const ids = context.files.map((file) => file.id)
    for (const witness of [
      '/LandingTitle.tsx',
      '/packages/components/dist/',
      '/packages/reset-css/dist/',
    ])
      expect(ids.some((id) => id.includes(witness))).toBe(true)
    console.info(
      JSON.stringify({
        case: 'landing-source-reservation',
        context: context.context,
        count: ids.length,
        elapsedMs: performance.now() - started,
      }),
    )
  }
  const foreign = /\/node_modules\/(?:react|next|webpack)\//
  expect(observed.some((path) => foreign.test(posix(path)))).toBe(false)
  expect(performance.now() - started).toBeLessThan(30000)
}, 30000)
