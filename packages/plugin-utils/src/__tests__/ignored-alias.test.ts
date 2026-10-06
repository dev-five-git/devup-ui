import { join } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import { buildStaticImportGraph, createModuleResolver } from '../import-graph'
import { ConfigLoadError } from '../load-config'
import { ModuleAliasPackageError, resolveModuleAlias } from '../module-alias'
import { collectNumberedFiles, seedFileNumbers } from '../numbering'
import { aliasFixture } from './alias-array-fixture'

let fixture: ReturnType<typeof aliasFixture>
beforeEach(() => {
  fixture = aliasFixture()
})
afterEach(() => fixture.dispose())

it.each(['provider', 'provider/feature'])(
  'ignores scalar false when request is %s',
  (request) => {
    const { root, entry, installed } = fixture
    const alias = { provider: false } as const
    expect(installed(request, { alias })).toBe(false)
    expect(createModuleResolver({ cwd: root, alias })(request, entry)).toEqual({
      ignored: true,
    })
  },
)

it.each([false, true])(
  'selects ordered false candidates when earlier file exists=%s',
  (exists) => {
    const { root, entry, file } = fixture
    const chosen = join(root, 'chosen.js')
    if (exists) file('chosen.js')
    const resolve = createModuleResolver({
      cwd: root,
      alias: { provider: [chosen, false] },
    })
    expect(resolve('provider', entry)).toEqual(
      exists ? { path: chosen, code: 'export {}' } : { ignored: true },
    )
  },
)

it('stops before later strings when false is first', () => {
  const { root, entry, file } = fixture
  const chosen = file('chosen.js')
  expect(
    createModuleResolver({ cwd: root, alias: { provider: [false, chosen] } })(
      'provider',
      entry,
    ),
  ).toEqual({ ignored: true })
})

it('does not claim public factory parity for mixed false arrays', () => {
  const { root, installed } = fixture
  expect(() =>
    installed('provider', {
      alias: { provider: [join(root, 'missing'), false] },
    }),
  ).toThrow(TypeError)
})

it('keeps exact false aliases from matching subpaths', () => {
  const { root, entry, installed } = fixture
  const alias = { provider$: false } as const
  expect(installed('provider', { alias })).toBe(false)
  expect(
    createModuleResolver({ cwd: root, alias })('provider/feature', entry),
  ).toBeUndefined()
})

it.each(['provider', 'provider/feature'])(
  're-enters scalar false chains for %s',
  (request) => {
    const { root, entry, installed } = fixture
    const alias = { provider: 'nested', nested: 'empty', empty: false } as const
    expect(installed(request, { alias })).toBe(false)
    expect(createModuleResolver({ cwd: root, alias })(request, entry)).toEqual({
      ignored: true,
    })
  },
)

it.each(['provider', 'provider/feature'])(
  'reaches false after equal/below-target guards for %s',
  (request) => {
    const { root, entry } = fixture
    expect(
      createModuleResolver({
        cwd: root,
        alias: { provider: ['provider', false] },
      })(request, entry),
    ).toEqual({ ignored: true })
  },
)

it('distinguishes excluded false from ignored success after a missing candidate', () => {
  const { entry } = fixture
  expect(
    resolveModuleAlias('provider', {
      importer: entry,
      alias: { provider: ['excluded', false] },
      resolveRequest: () => false,
    }),
  ).toEqual({ ignored: true })
})

it('keeps fatal cycles before a later false candidate', () => {
  const { root, entry } = fixture
  expect(() =>
    createModuleResolver({
      cwd: root,
      alias: { provider: ['nested', false], nested: 'provider' },
    })('provider', entry),
  ).toThrow('Module alias cycle')
})

it('reaches false after a complete nested ordinary miss', () => {
  const { root, entry } = fixture
  expect(
    createModuleResolver({
      cwd: root,
      alias: { provider: ['nested', false], nested: join(root, 'missing') },
    })('provider', entry),
  ).toEqual({ ignored: true })
})

it.each(['exports', 'manifest'])(
  'preserves fatal %s faults before a false candidate',
  (kind) => {
    const { root, entry, file } = fixture
    file(
      'node_modules/broken/package.json',
      kind === 'exports' ? '{"exports":{"browser":"./missing.js"}}' : '{',
    )
    expect(() =>
      createModuleResolver({
        cwd: root,
        alias: { provider: ['broken', false] },
      })('provider', entry),
    ).toThrow(kind === 'exports' ? ModuleAliasPackageError : ConfigLoadError)
  },
)

it('keeps directory exclusion false separate from ordinary misses', () => {
  const { entry } = fixture
  expect(
    resolveModuleAlias('provider', {
      importer: entry,
      alias: { provider: ['excluded'] },
      resolveRequest: () => false,
    }),
  ).toBe(false)
  expect(() =>
    resolveModuleAlias('provider', {
      importer: entry,
      alias: { provider: ['missing', 'excluded'] },
      resolveRequest: (name) => (name === 'excluded' ? false : undefined),
    }),
  ).toThrow('cannot resolve candidates')
})

it('seeds only physically scanned source files when graph requests are ignored', () => {
  const { root, entry, file } = fixture
  const token = file('src/token.ts')
  file('src/main.ts', "import 'provider';")
  const graph = buildStaticImportGraph('src', undefined, {
    cwd: root,
    alias: { provider: false },
  })
  const files = collectNumberedFiles({
    roots: [join(root, 'src')],
    toId: (path) => path,
  })
  const seeded: string[][] = []
  seedFileNumbers({ seedFileMap: (paths) => seeded.push(paths) }, files)
  expect(graph.files).toEqual([entry, token].sort())
  expect(seeded).toEqual([[entry, token].sort()])
})

it.each(['absent', 'root', 'src'])(
  'resolves stock-shaped instrumentation chain when file is %s',
  (location) => {
    const { root, entry, file, installed } = fixture
    const request = 'private-next-instrumentation-client-user'
    const alias = {
      [request]: [
        join(root, 'src/instrumentation-client'),
        join(root, 'instrumentation-client'),
        'private-next-empty-module',
      ],
      'private-next-empty-module': false,
    } as const
    const expected =
      location === 'absent'
        ? undefined
        : file(`${location === 'src' ? 'src/' : ''}instrumentation-client.js`)
    expect(installed(request, { alias })).toBe(expected ?? false)
    expect(createModuleResolver({ cwd: root, alias })(request, entry)).toEqual(
      expected ? { path: expected, code: 'export {}' } : { ignored: true },
    )
  },
)

it('retries earlier candidates after a previously ignored resolution', () => {
  const { root, entry, file } = fixture
  const resolve = createModuleResolver({
    cwd: root,
    alias: { provider: [join(root, 'earlier'), 'empty'], empty: false },
  })
  expect(resolve('provider', entry)).toEqual({ ignored: true })
  const path = file('earlier.js')
  expect(resolve('provider', entry)).toEqual({ path, code: 'export {}' })
})

it('bypasses preparation and numbering IDs for ignored requests', () => {
  const { root, entry } = fixture
  const seen: string[] = []
  const resolve = createModuleResolver({
    cwd: root,
    alias: { provider: false },
    prepareSource: (path) => {
      seen.push(path)
      return ''
    },
    toId: (path) => {
      seen.push(path)
      return path
    },
  })
  expect(resolve('provider', entry)).toEqual({ ignored: true })
  expect(seen).toEqual([])
})

it.each([false, true])(
  'omits ignored static/dynamic/reexport edges when prepared=%s',
  async (prepared) => {
    const { root, entry, file } = fixture
    file(
      'src/main.ts',
      "import 'provider'; import('provider/feature'); export * from 'provider';",
    )
    const options = { cwd: root, alias: { provider: false } } as const
    const graph = prepared
      ? await buildStaticImportGraph('src', undefined, {
          ...options,
          prepareSource: () =>
            "import 'provider'; import('provider/feature'); export * from 'provider';",
        })
      : buildStaticImportGraph('src', undefined, options)
    expect(graph.files).toEqual([entry])
    expect([...(graph.staticImports.get(entry) ?? [])]).toEqual([])
    expect([...(graph.dynamicImports.get(entry) ?? [])]).toEqual([])
    expect([...(graph.externalImports.get(entry) ?? [])]).toEqual([])
  },
)
