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
  'selects the first fully resolving candidate when first exists=%s',
  (firstExists) => {
    const { root, entry, file, installed } = fixture
    const first = join(root, 'first/index.js')
    if (firstExists) file('first/index.js', "export const value = 'first'")
    const second = file('second/index.js', "export const value = 'second'")
    file('src/main.ts', "import 'provider'")
    const alias = { provider: [dirname(first), dirname(second)] }
    const expected = firstExists ? first : second
    const resolved = createModuleResolver({ cwd: root, alias })(
      'provider',
      entry,
    )
    expect(resolved).toEqual({
      path: expected,
      code: firstExists
        ? "export const value = 'first'"
        : "export const value = 'second'",
    })
    expect(installed('provider', { alias })).toBe(expected)
    expect(
      buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
    ).toEqual([entry, expected].sort())
  },
)

it.each(['provider', 'provider/feature'])(
  'applies exact and subpath arrays when request is %s',
  (request) => {
    const { root, entry, file, installed } = fixture
    const exact = file('exact.js')
    const feature = file('second/feature.js')
    file('src/main.ts', `import '${request}'`)
    const alias = {
      provider$: [join(root, 'missing'), exact],
      provider: [join(root, 'absent'), dirname(feature)],
    }
    const expected = request === 'provider' ? exact : feature
    expect(installed(request, { alias })).toBe(expected)
    expect(
      createModuleResolver({ cwd: root, alias })(request, entry)?.path,
    ).toBe(expected)
    expect(
      buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
    ).toEqual([entry, expected].sort())
  },
)

it('reports all candidates at the real importer without original or lateral fallback', () => {
  const { root, entry, file, installed } = fixture
  file('node_modules/original/package.json', '{"main":"index.js"}')
  file('node_modules/original/index.js')
  const later = file('later.js')
  file('src/main.ts', "import 'original'")
  const candidates = [join(root, 'missing'), join(root, 'absent')]
  const alias = { original: candidates, original$: [later] }
  expect(() => installed('original', { alias })).toThrow()
  const message = `${entry}:1:1: Module alias original cannot resolve candidates ${JSON.stringify(candidates)}`
  expect(() =>
    createModuleResolver({ cwd: root, alias })('original', entry),
  ).toThrow(message)
  expect(() =>
    buildStaticImportGraph('src', undefined, { cwd: root, alias }),
  ).toThrow(message)
})

it('continues to the next key when a matched candidate list is empty', () => {
  const { root, entry, file, installed } = fixture
  file('src/main.ts', "import 'provider'")
  const chosen = file('later.js')
  const alias = { provider: [], provider$: chosen }
  expect(installed('provider', { alias })).toBe(chosen)
  expect(
    createModuleResolver({ cwd: root, alias })('provider', entry)?.path,
  ).toBe(chosen)
  expect(
    buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
  ).toEqual([entry, chosen].sort())
})

it('tries another candidate after a complete nested alias miss', () => {
  const { root, entry, file, installed } = fixture
  const chosen = file('chosen.js')
  file('src/main.ts', "import 'provider'")
  const alias = { provider: ['nested', chosen], nested: join(root, 'missing') }
  expect(installed('provider', { alias })).toBe(chosen)
  expect(
    createModuleResolver({ cwd: root, alias })('provider', entry)?.path,
  ).toBe(chosen)
  expect(
    buildStaticImportGraph('src', undefined, { cwd: root, alias }).files,
  ).toEqual([entry, chosen].sort())
})

it('does not match an exact array key against a package subpath', () => {
  const { root, entry, file, installed } = fixture
  file('src/main.ts', "import 'provider/feature'")
  file(
    'node_modules/provider/package.json',
    '{"exports":{"./feature":"./feature.js"}}',
  )
  const chosen = file('node_modules/provider/feature.js')
  const alias = { provider$: [file('wrong.js')] }
  expect(installed('provider/feature', { alias })).toBe(chosen)
  expect(
    createModuleResolver({ cwd: root, alias })('provider/feature', entry)?.path,
  ).toBe(chosen)
  expect(
    buildStaticImportGraph('src', undefined, {
      cwd: root,
      alias,
      include: ['provider'],
    }).files,
  ).toEqual([entry, chosen].sort())
})
