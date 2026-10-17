import { dirname, join } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import { buildStaticImportGraph, createModuleResolver } from '../import-graph'
import { aliasFixture } from './alias-array-fixture'

let fixture: ReturnType<typeof aliasFixture>
beforeEach(() => {
  fixture = aliasFixture()
})
afterEach(() => fixture.dispose())

it.each([false, true])(
  'selects the first native descriptor success when earlier file exists=%s',
  (exists) => {
    const { root, entry, file, installed } = fixture
    const first = join(root, 'first.js')
    if (exists) file('first.js')
    const second = file('second.js')
    file('src/main.ts', "import 'provider'")
    const alias = [
      { name: 'provider', alias: [first, second] },
      { name: 'provider', alias: false },
    ] as const
    const expected = exists ? first : second
    expect(installed('provider', { alias })).toBe(expected)
    expect(
      createModuleResolver({ cwd: root, alias })('provider', entry)?.path,
    ).toBe(expected)
    expect(
      buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
    ).toEqual([entry, expected].sort())
  },
)

it.each([false, true])(
  'keeps a rewriting miss terminal despite a duplicate later ignored=%s',
  (ignored) => {
    const { root, entry, file, installed } = fixture
    const missing = join(root, 'missing.js')
    const later = file('later.js')
    file('src/main.ts', "import 'provider'")
    const alias = [
      { name: 'provider', alias: missing },
      { name: 'provider', alias: ignored ? false : later },
    ] as const
    expect(() => installed('provider', { alias })).toThrow()
    const message = `${entry}:1:1: Module alias provider cannot resolve candidates ${JSON.stringify([missing])}`
    expect(() =>
      createModuleResolver({ cwd: root, alias })('provider', entry),
    ).toThrow(message)
    expect(() =>
      buildStaticImportGraph('src', undefined, { cwd: root, alias }),
    ).toThrow(message)
  },
)

it.each(['empty', 'self'])(
  'advances to a duplicate descriptor when the first is %s without a rewrite',
  (kind) => {
    const { root, entry, file, installed } = fixture
    const chosen = file('chosen.js')
    file('src/main.ts', "import 'provider'")
    const alias = [
      { name: 'provider', alias: kind === 'empty' ? [] : ['provider'] },
      { name: 'provider', alias: chosen },
    ] as const
    expect(installed('provider', { alias })).toBe(chosen)
    expect(
      createModuleResolver({ cwd: root, alias })('provider', entry)?.path,
    ).toBe(chosen)
    expect(
      buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
    ).toEqual([entry, chosen].sort())
  },
)

it.each(['empty-map', 'empty-descriptor', 'self', 'below-target', 'disabled'])(
  'uses the raw real file when %s makes no rewrite',
  (kind) => {
    const { root, entry, file, installed } = fixture
    file('node_modules/provider/package.json', '{"main":"index.js"}')
    const request = kind === 'below-target' ? 'provider/child' : 'provider'
    const chosen = file(
      kind === 'below-target'
        ? 'node_modules/provider/child.js'
        : 'node_modules/provider/index.js',
    )
    file('src/main.ts', `import '${request}'`)
    const alias =
      kind === 'empty-map'
        ? { provider: [] }
        : kind === 'disabled'
          ? false
          : [
              {
                name: 'provider',
                alias: kind === 'empty-descriptor' ? [] : 'provider',
              },
            ]
    expect(installed(request, { alias })).toBe(chosen)
    expect(
      createModuleResolver({ cwd: root, alias })(request, entry)?.path,
    ).toBe(chosen)
    expect(
      buildStaticImportGraph('src', undefined, {
        cwd: root,
        alias,
        include: ['provider'],
      }).files,
    ).toEqual([entry, chosen].sort())
  },
)

it.each(['provider$', 'provider$/child'])(
  'keeps a literal dollar prefix when onlyModule=false and request=%s',
  (request) => {
    const { root, entry, file, installed } = fixture
    file('src/main.ts', `import '${request}'`)
    const alias = [
      { name: 'provider$', alias: false, onlyModule: false },
    ] as const
    expect(installed(request, { alias })).toBe(false)
    expect(createModuleResolver({ cwd: root, alias })(request, entry)).toEqual({
      ignored: true,
    })
    expect(
      buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
    ).toEqual([entry])
  },
)

it.each(['provider', 'provider/child'])(
  'matches onlyModule=true like map trailing dollar for %s',
  (request) => {
    const { root, entry, file, installed } = fixture
    file('src/main.ts', `import '${request}'`)
    const chosen = file('target/child.js')
    const alias = [
      { name: 'provider', alias: false, onlyModule: true },
      { name: 'provider', alias: dirname(chosen) },
    ] as const
    const map = { provider$: false, provider: dirname(chosen) } as const
    const expected =
      request === 'provider'
        ? { ignored: true }
        : { path: chosen, code: 'export {}' }
    expect(installed(request, { alias })).toBe(
      request === 'provider' ? false : chosen,
    )
    expect(createModuleResolver({ cwd: root, alias })(request, entry)).toEqual(
      expected,
    )
    expect(
      createModuleResolver({ cwd: root, alias: map })(request, entry),
    ).toEqual(expected)
  },
)

it('re-enters descriptors through finite chains to an ignored module', () => {
  const { root, entry, file, installed } = fixture
  file('src/main.ts', "import 'provider'")
  const alias = [
    { name: 'provider', alias: ['missing', 'empty'] },
    { name: 'empty', alias: false },
  ] as const
  expect(installed('provider', { alias })).toBe(false)
  expect(createModuleResolver({ cwd: root, alias })('provider', entry)).toEqual(
    { ignored: true },
  )
  expect(
    buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
  ).toEqual([entry])
})

it.each([true, false])(
  'uses reached false candidate semantics while native Factory rejects mixed arrays; first false=%s',
  (first) => {
    const { root, entry, file, installed } = fixture
    const chosen = file('chosen.js')
    const alias = [
      {
        name: 'provider',
        alias: first ? [false, chosen] : [join(root, 'missing'), false],
      },
    ] as const
    expect(() => installed('provider', { alias })).toThrow(TypeError)
    expect(
      createModuleResolver({ cwd: root, alias })('provider', entry),
    ).toEqual({ ignored: true })
  },
)

it.each([false, true])(
  'reports unsupported wildcard alias names instead of dropping their native form; descriptor=%s',
  (descriptor) => {
    const { root, entry } = fixture
    const alias = descriptor
      ? ([{ name: 'provider/*', alias: false }] as const)
      : ({ 'provider/*': false } as const)
    expect(() =>
      createModuleResolver({ cwd: root, alias })('provider/child', entry),
    ).toThrow(`${entry}:1:1: Module alias provider/* cannot use wildcard names`)
  },
)
