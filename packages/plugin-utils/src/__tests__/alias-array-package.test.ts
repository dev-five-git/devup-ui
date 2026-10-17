import { dirname, join } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import { buildStaticImportGraph, createModuleResolver } from '../import-graph'
import { aliasFixture } from './alias-array-fixture'

let fixture: ReturnType<typeof aliasFixture>
beforeEach(() => {
  fixture = aliasFixture()
})
afterEach(() => fixture.dispose())

it.each([
  { conditions: ['browser', 'import'] },
  { conditions: ['import'] },
  { conditions: [] },
])(
  'resolves active conditions before selecting the package (%j)',
  ({ conditions }: { readonly conditions: readonly string[] }) => {
    const { root, entry, file, installed } = fixture
    file('src/main.ts', "import 'provider'")
    file('node_modules/missing-main/package.json', '{"main":"absent.js"}')
    file(
      'node_modules/actual/package.json',
      '{"exports":{"browser":"./browser.js","import":"./import.js","default":"./default.js"}}',
    )
    const browser = file('node_modules/actual/browser.js', "import './leaf'")
    const imported = file('node_modules/actual/import.js', "import './leaf'")
    const fallback = file('node_modules/actual/default.js', "import './leaf'")
    const leaf = file('node_modules/actual/leaf.js')
    const alias = { provider: ['missing-main', 'actual'] }
    const expected = conditions.includes('browser')
      ? browser
      : conditions.includes('import')
        ? imported
        : fallback
    expect(installed('provider', { alias, conditions })).toBe(expected)
    expect(
      createModuleResolver({ cwd: root, alias, conditions })('provider', entry),
    ).toEqual({ path: expected, code: "import './leaf'" })
    expect(
      buildStaticImportGraph('src', undefined, {
        cwd: root,
        alias,
        conditions,
        include: ['actual'],
      }).files,
    ).toEqual([entry, expected, leaf].sort())
    expect(
      buildStaticImportGraph('src', undefined, { cwd: root, alias, conditions })
        .files,
    ).toEqual([entry])
  },
)

it.each(['blocked', 'condition', 'poison'])(
  'preserves fatal %s package errors instead of trying a later candidate',
  (kind) => {
    const { root, entry, file, installed } = fixture
    file('src/main.ts', "import 'provider'")
    const manifest =
      kind === 'blocked'
        ? '{"exports":{"./other":"./index.js"}}'
        : kind === 'condition'
          ? '{"exports":{"browser":"./absent.js","default":"./index.js"}}'
          : '{'
    const manifestFile = file('node_modules/first/package.json', manifest)
    file('node_modules/first/index.js')
    const alias = { provider: ['first', file('later.js')] }
    expect(() => installed('provider', { alias })).toThrow()
    const expected =
      kind === 'poison'
        ? manifestFile
        : `${entry}:1:1: Module alias package first`
    expect(() =>
      createModuleResolver({ cwd: root, alias, conditions: ['browser'] })(
        'provider',
        entry,
      ),
    ).toThrow(expected)
    expect(() =>
      buildStaticImportGraph('src', undefined, {
        cwd: root,
        alias,
        conditions: ['browser'],
      }),
    ).toThrow(expected)
  },
)

it('keeps cycles fatal even with a resolving later candidate', () => {
  const { root, entry, file, installed } = fixture
  file('src/main.ts', "import 'provider'")
  const alias = { provider: ['nested', file('later.js')], nested: 'provider' }
  expect(() => installed('provider', { alias })).toThrow()
  expect(() =>
    createModuleResolver({ cwd: root, alias })('provider', entry),
  ).toThrow(`${entry}:1:1: Module alias cycle`)
  expect(() =>
    buildStaticImportGraph('src', undefined, { cwd: root, alias }),
  ).toThrow(`${entry}:1:1: Module alias cycle`)
})

it('skips self candidates while retaining ordered overlapping keys', () => {
  const { root, entry, file, installed } = fixture
  const chosen = file('chosen.js')
  file('src/main.ts', "import 'provider'")
  const alias = {
    provider: ['provider', chosen],
    provider$: [file('later.js')],
  }
  expect(installed('provider', { alias })).toBe(chosen)
  expect(
    createModuleResolver({ cwd: root, alias })('provider', entry)?.path,
  ).toBe(chosen)
  expect(
    buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
  ).toEqual([entry, chosen].sort())
})

it('follows a selected absolute package candidate and its closure', () => {
  const { root, entry, file, installed } = fixture
  file('src/main.ts', "import 'provider'")
  file('node_modules/actual/package.json', '{"main":"index.js"}')
  const chosen = file('node_modules/actual/index.js', "import './leaf'")
  const leaf = file('node_modules/actual/leaf.js')
  const alias = { provider: [join(root, 'missing'), chosen] }
  expect(installed('provider', { alias })).toBe(chosen)
  expect(
    createModuleResolver({ cwd: root, alias })('provider', entry)?.path,
  ).toBe(chosen)
  expect(
    buildStaticImportGraph('src', undefined, {
      cwd: root,
      alias,
      include: ['actual'],
    }).files,
  ).toEqual([entry, chosen, leaf].sort())
})

it.each([false, true])(
  'skips poisoned excluded candidates without external prewarming (allExcluded=%s)',
  (allExcluded) => {
    const { root, entry, file } = fixture
    file('src/main.ts', "import 'provider'")
    const poison = file('node_modules/poison/package.json', '{')
    const chosen = file('local/provider.js', "import './leaf'")
    const leaf = file('local/leaf.js')
    const alias = { provider: ['poison', chosen] }
    const exclude = allExcluded
      ? [dirname(poison), dirname(chosen)]
      : [dirname(poison)]
    const graph = buildStaticImportGraph('src', undefined, {
      cwd: root,
      alias,
      exclude,
    })
    expect(graph.files).toEqual(
      (allExcluded ? [entry] : [entry, chosen, leaf]).sort(),
    )
    expect(graph.externalImports?.get(entry)).toEqual(new Set())
  },
)

it('keeps a mixed excluded and missing candidate list terminal', () => {
  const { root, entry, file } = fixture
  file('src/main.ts', "import 'provider'")
  const poison = file('node_modules/poison/package.json', '{')
  const alias = { provider: ['poison', join(root, 'missing')] }
  expect(() =>
    buildStaticImportGraph('src', undefined, {
      cwd: root,
      alias,
      exclude: [dirname(poison)],
    }),
  ).toThrow(`${entry}:1:1: Module alias provider`)
})
