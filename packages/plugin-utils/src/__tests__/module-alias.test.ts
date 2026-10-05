import * as fs from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import { buildStaticImportGraph, createModuleResolver } from '../import-graph'

const installedRequire = createRequire(
  resolve(
    'node_modules/.bun/enhanced-resolve@5.25.1/node_modules/enhanced-resolve/package.json',
  ),
)
const enhancedResolve = installedRequire('enhanced-resolve')
let root: string
let entry: string
beforeEach(() => {
  const parent = join(
    tmpdir(),
    'opencode',
    'workers',
    'w20-plugins-core',
    'shared-api',
  )
  fs.mkdirSync(parent, { recursive: true })
  root = fs.realpathSync.native(fs.mkdtempSync(join(parent, 'alias-')))
  entry = file('src/main.ts')
})
afterEach(() => fs.rmSync(root, { recursive: true, force: true }))
function file(path: string, source = 'export {}'): string {
  const target = join(root, path)
  fs.mkdirSync(dirname(target), { recursive: true })
  fs.writeFileSync(target, source)
  return target
}

function installed(
  request: string,
  alias: Readonly<Record<string, string>>,
  conditions: readonly string[] = ['import', 'browser'],
): string | undefined {
  const resolver = enhancedResolve.ResolverFactory.createResolver({
    fileSystem: fs,
    useSyncFileSystemCalls: true,
    extensions: ['.js'],
    alias,
    conditionNames: conditions,
  })
  try {
    return resolver.resolveSync({}, dirname(entry), request) || undefined
  } catch (cause) {
    if (cause instanceof Error && /Can't resolve/.test(cause.message))
      return undefined
    throw cause
  }
}

it('matches installed resolver declaration order, exact keys and subpath boundaries', () => {
  const broad = file('broad/index.js')
  const broadSub = file('broad/feature.js')
  const specific = file('specific.js')
  const dir = dirname(broad)
  const cases = [
    {
      request: '@x/feature',
      alias: { '@x': dir, '@x/feature': specific },
      expected: broadSub,
    },
    {
      request: '@x/feature',
      alias: { '@x/feature': specific, '@x': dir },
      expected: specific,
    },
    {
      request: '@x',
      alias: { '@x$': specific, '@x': dir },
      expected: specific,
    },
    {
      request: '@x/feature',
      alias: { '@x$': specific, '@x': dir },
      expected: broadSub,
    },
    { request: '@xyz', alias: { '@x': dir }, expected: undefined },
    { request: '@x', alias: { '@x': dir }, expected: broad },
  ]
  for (const { request, alias, expected } of cases) {
    expect(installed(request, alias)).toBe(expected)
    expect(
      createModuleResolver({ cwd: root, alias })(request, entry)?.path,
    ).toBe(expected)
  }
})

it.each([
  { conditions: ['browser', 'import'] },
  { conditions: ['import'] },
  { conditions: [] },
])(
  'matches package exports conditions after alias rewrite (%j)',
  ({ conditions }) => {
    file(
      'node_modules/target/package.json',
      JSON.stringify({
        exports: {
          '.': {
            browser: './browser.js',
            import: './import.js',
            default: './default.js',
          },
          './feature': './feature.js',
        },
      }),
    )
    const browser = file('node_modules/target/browser.js')
    const imported = file('node_modules/target/import.js')
    const fallback = file('node_modules/target/default.js')
    const feature = file('node_modules/target/feature.js')
    const alias = { '@x': 'target' }
    const expected = conditions.includes('browser')
      ? browser
      : conditions.includes('import')
        ? imported
        : fallback
    expect(installed('@x', alias, conditions)).toBe(expected)
    expect(
      createModuleResolver({ cwd: root, alias, conditions })('@x', entry)?.path,
    ).toBe(expected)
    expect(installed('@x/feature', alias, conditions)).toBe(feature)
    expect(
      createModuleResolver({ cwd: root, alias, conditions })(
        '@x/feature',
        entry,
      )?.path,
    ).toBe(feature)
  },
)

it('matches missing-target terminal failure even when the original and a later alias exist', () => {
  file('node_modules/original/package.json', '{"main":"index.js"}')
  file('node_modules/original/index.js')
  const later = file('later.js')
  const alias = { original: join(root, 'missing'), original$: later }
  expect(installed('original', alias)).toBeUndefined()
  expect(() =>
    createModuleResolver({ cwd: root, alias })('original', entry),
  ).toThrow(
    `${entry}:1:1: Module alias original cannot resolve candidates ${JSON.stringify([alias.original])}`,
  )
})

it('matches finite chains and self-alias skips', () => {
  const target = file('target.js')
  const alias = { '@a': '@a', '@a/feature': '@b', '@b': target }
  expect(installed('@a/feature', alias)).toBe(target)
  expect(
    createModuleResolver({ cwd: root, alias })('@a/feature', entry)?.path,
  ).toBe(target)
})

it('fails alias cycles at the real importer location', () => {
  const alias = { '@a': '@b', '@b': '@a' }
  expect(() => installed('@a', alias)).toThrow()
  expect(() => createModuleResolver({ cwd: root, alias })('@a', entry)).toThrow(
    `${entry}:1:1`,
  )
})

it('matches installed self-target subpath guards without an unbounded rewrite', () => {
  file(
    'node_modules/pkg/package.json',
    '{"exports":{"./feature":"./feature.js"}}',
  )
  const feature = file('node_modules/pkg/feature.js')
  const alias = { pkg: 'pkg/feature' }
  expect(installed('pkg', alias)).toBe(feature)
  expect(createModuleResolver({ cwd: root, alias })('pkg', entry)?.path).toBe(
    feature,
  )
})

it('follows the actual aliased included-package closure rather than the alias name', () => {
  fs.writeFileSync(entry, "import 'provider'")
  file('node_modules/actual/package.json', '{"main":"index.js"}')
  const provider = file(
    'node_modules/actual/index.js',
    "import './nested/value.generated'",
  )
  const leaf = file('node_modules/actual/nested/value.generated.mjs')
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    alias: { provider: 'actual' },
    include: ['actual'],
  })
  expect(graph.files).toEqual([entry, provider, leaf].sort())
  expect(
    buildStaticImportGraph('src', undefined, {
      cwd: root,
      alias: { provider: 'actual' },
    }).files,
  ).toEqual([entry])
})

it('follows an absolute alias into the actual included package and honors its exclusions', () => {
  fs.writeFileSync(entry, "import 'provider'")
  file('node_modules/actual/package.json', '{"main":"index.js"}')
  const provider = file(
    'node_modules/actual/index.js',
    "import './nested/leaf'",
  )
  const leaf = file('node_modules/actual/nested/leaf.mjs')
  const options = { cwd: root, alias: { provider }, include: ['actual'] }
  expect(buildStaticImportGraph('src', undefined, options).files).toEqual(
    [entry, provider, leaf].sort(),
  )
  expect(
    buildStaticImportGraph('src', undefined, {
      ...options,
      exclude: [dirname(leaf)],
    }).files,
  ).toEqual([entry, provider].sort())
})
