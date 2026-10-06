import { readFileSync, realpathSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import { buildStaticImportGraph, createModuleResolver } from '../import-graph'
import { ConfigLoadError } from '../load-config'
import { aliasFixture } from './alias-array-fixture'

let fixture: ReturnType<typeof aliasFixture>
beforeEach(() => {
  fixture = aliasFixture()
})
afterEach(() => fixture.dispose())

it.each([
  { conditions: ['import'] },
  { conditions: ['require'] },
  { conditions: [] },
])(
  'reads the installed Next SWC absolute directory module regardless of conditions=%j',
  ({ conditions }) => {
    // Given the actual helper installation resolved from this benchmark's Next.
    const { root, entry, file, installed } = fixture
    const next = createRequire(
      resolve('benchmark/next-devup-ui/package.json'),
    ).resolve('next/package.json')
    const helpers = dirname(
      createRequire(next).resolve('@swc/helpers/package.json'),
    )
    const request = '@swc/helpers/_/_interop_require_wildcard'
    const alias = { '@swc/helpers/_': join(helpers, '_') }
    const expected = realpathSync(
      join(helpers, 'esm/_interop_require_wildcard.js'),
    )
    file('src/main.ts', `import '${request}'`)
    expect(
      installed(request, { alias, conditions, mainFields: ['module', 'main'] }),
    ).toBe(expected)
    expect(
      installed(request, { alias, conditions, mainFields: ['main'] }),
    ).toBe(realpathSync(join(helpers, 'cjs/_interop_require_wildcard.cjs')))
    // When the shared resolver and graph follow the same real directory alias.
    const selected = createModuleResolver({ cwd: root, alias, conditions })(
      request,
      entry,
    )
    const graph = buildStaticImportGraph('src', undefined, {
      cwd: root,
      alias,
      conditions,
      include: ['@swc/helpers'],
    })
    // Then both match native module-first resolution and the installed code.
    expect(selected).toEqual({
      path: expected,
      code: readFileSync(expected, 'utf-8'),
    })
    expect(graph.staticImports.get(entry)).toEqual(new Set([expected]))
    expect(graph.files).toEqual([entry, expected].sort())
  },
)

it.each(['absolute', 'relative'])(
  'uses directory main fields before index and ignores exports when alias is %s',
  (kind) => {
    const { root, entry, file, installed } = fixture
    file('src/main.ts', "import 'provider'")
    file(
      'target/package.json',
      JSON.stringify({
        module: './module.js',
        main: './main.js',
        exports: { import: './export.js', require: './absent.js' },
      }),
    )
    const chosen = file('target/module.js', "import './leaf'")
    file('target/main.js')
    file('target/index.js')
    file('target/export.js')
    const leaf = file('target/leaf.js')
    const alias = {
      provider: kind === 'absolute' ? join(root, 'target') : '../target',
    }
    expect(
      installed('provider', { alias, mainFields: ['module', 'main'] }),
    ).toBe(chosen)
    const selected = createModuleResolver({ cwd: root, alias })(
      'provider',
      entry,
    )
    const graph = buildStaticImportGraph('src', undefined, { cwd: root, alias })
    expect(selected).toEqual({ path: chosen, code: "import './leaf'" })
    expect(graph.files).toEqual([entry, chosen, leaf].sort())
  },
)

it.each(['main', 'index', 'self', 'nested', 'extension', 'file'])(
  'matches native directory entry fallback when selected target is %s',
  (kind) => {
    const { root, entry, file, installed } = fixture
    file('src/main.ts', "import 'provider'")
    file(
      'target/package.json',
      JSON.stringify({
        module:
          kind === 'self' ? '.' : kind === 'nested' ? './child' : './absent',
        main: kind === 'index' || kind === 'self' ? './absent-main' : './main',
      }),
    )
    const chosen = file(
      kind === 'file'
        ? 'target.js'
        : kind === 'nested'
          ? 'target/child/index.js'
          : kind === 'index' || kind === 'self'
            ? 'target/index.js'
            : 'target/main.js',
      'export const selected = 42',
    )
    const alias = { provider: join(root, 'target') }
    expect(
      installed('provider', { alias, mainFields: ['module', 'main'] }),
    ).toBe(chosen)
    const selected = createModuleResolver({ cwd: root, alias })(
      'provider',
      entry,
    )
    expect(selected).toEqual({
      path: chosen,
      code: 'export const selected = 42',
    })
    expect(
      buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
    ).toEqual([entry, chosen].sort())
  },
)

it.each(['{', '[]', 'null'])(
  'keeps malformed directory manifests fatal before a later false (%s)',
  (content) => {
    const { root, entry, file, installed } = fixture
    file('src/main.ts', "import 'provider'")
    const manifest = file('target/package.json', content)
    file('target/index.js')
    const target = join(root, 'target')
    if (content === '[]') {
      expect(installed('provider', { alias: { provider: target } })).toBe(
        join(target, 'index.js'),
      )
    } else {
      expect(() =>
        installed('provider', { alias: { provider: target } }),
      ).toThrow()
    }
    const alias = { provider: [target, false] }
    expect(() =>
      createModuleResolver({ cwd: root, alias })('provider', entry),
    ).toThrow(ConfigLoadError)
    expect(() =>
      buildStaticImportGraph('src', undefined, { cwd: root, alias }),
    ).toThrow(`${manifest}:1:1: Cannot load configuration`)
  },
)

it('keeps directory entry cycles fatal and located instead of reaching false', () => {
  const { root, entry, file, installed } = fixture
  file('src/main.ts', "import 'provider'")
  const manifest = file('target/package.json', '{"module":"./child"}')
  file('target/child/package.json', '{"module":".."}')
  const target = join(root, 'target')
  expect(() =>
    installed('provider', {
      alias: { provider: target },
      mainFields: ['module', 'main'],
    }),
  ).toThrow()
  const alias = { provider: [target, false] }
  expect(() =>
    createModuleResolver({ cwd: root, alias })('provider', entry),
  ).toThrow(`${manifest}:1:1:`)
  expect(() =>
    buildStaticImportGraph('src', undefined, { cwd: root, alias }),
  ).toThrow(`${manifest}:1:1:`)
})

it.each([false, true])(
  'skips excluded poisoned directory manifests before IO (all excluded=%s)',
  (all) => {
    const { root, entry, file } = fixture
    file('src/main.ts', "import 'provider'")
    const poison = dirname(file('poison/package.json', '{'))
    const chosen = file('target/entry.js')
    const target = dirname(file('target/package.json', '{"main":"entry.js"}'))
    const alias = { provider: [poison, target] }
    const graph = buildStaticImportGraph('src', undefined, {
      cwd: root,
      alias,
      exclude: all ? [poison, target] : [poison],
    })
    expect(graph.files).toEqual((all ? [entry] : [entry, chosen]).sort())
    expect(graph.externalImports?.get(entry)).toEqual(new Set())
  },
)

it('excludes the selected main directory before reading its poisoned manifest', () => {
  const { root, entry, file } = fixture
  file('src/main.ts', "import 'provider'")
  const target = dirname(file('target/package.json', '{"module":"../poison"}'))
  const poison = dirname(file('poison/package.json', '{'))
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    alias: { provider: target },
    exclude: [poison],
  })
  expect(graph.files).toEqual([entry])
  expect(graph.externalImports?.get(entry)).toEqual(new Set())
})
