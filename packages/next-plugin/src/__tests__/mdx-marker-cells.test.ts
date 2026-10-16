import { mkdirSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { expect, it } from 'bun:test'

import { admitMdxNativeCell } from '../mdx-native-scope'
import { isMdxRecord } from '../mdx-pipeline'
import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { markerFixture } from './mdx-marker-fixture'
import { nativePreparationContext } from './mdx-safety-native'

it.each([false, true])(
  'matches the direct installed marker entry and repeat maps=%s',
  async (sourceMap) => {
    // Given
    const f = markerFixture('@mdx-js/loader', sourceMap)
    const options = { format: 'mdx', providerImportSource: f.filename }
    const original: unknown = createRequire(f.loader.loader)(f.loader.loader)
    if (typeof original !== 'function')
      throw new TypeError('missing direct installed loader')
    const expected = await new Promise<{
      readonly source: string
      readonly map: unknown
    }>((done, reject) =>
      original.call(
        {
          context: f.root,
          resourcePath: f.filename,
          sourceMap,
          getOptions: () => options,
          async: () => (error: unknown, source: Buffer, map: unknown) =>
            error ? reject(error) : done({ source: source.toString(), map }),
        },
        readFileSync(f.filename, 'utf8'),
      ),
    )
    // When
    const request = {
      ...f,
      pipeline: { ...f.pipeline, loaders: [{ ...f.loader, options }] },
    }
    const first = await compileMdx(request)
    const repeated = await compileMdx(request)
    // Then
    expect<unknown>({ source: first.source, map: first.map }).toEqual(expected)
    expect(repeated.source).toBe(first.source)
    expect(repeated.map).toEqual(first.map)
  },
)

it.each([false, true])(
  'keeps local native/private marker priming disjoint privateFirst=%s',
  async (privateFirst) => {
    // Given
    const f = markerFixture()
    const other = join(f.root, 'other.cjs')
    writeFileSync(
      other,
      "module.exports = function() { return function(tree) { tree.children[0].children[0].value = 'native-only' } }",
    )
    if (privateFirst) await f.prepare('private-only')
    else await f.native('private-only', other)
    // When
    const output = await f.prepare('private-only')
    const native = await f.native('private-only', other)
    // Then
    expect(output.source).toContain('private-only')
    expect(native.source).toContain('native-only')
  },
)

it.each([true, false])(
  'matches local installed marker bytes/maps and reuses one private H processor maps=%s',
  async (maps) => {
    // Given
    const f = markerFixture('@next/mdx/mdx-js-loader', maps)
    const native = await f.native('marker-a')
    expect(f.count()).toBe(1)
    // When
    const first = await f.prepare('marker-a')
    const again = await f.prepare('marker-a')
    // Then
    expect<unknown>({ source: first.source, map: first.map }).toEqual(native)
    expect(again.source).toBe(first.source)
    expect(again.map).toEqual(first.map)
    expect(f.count()).toBe(2)
  },
)

it('reuses local A after serializable A-B-A changes without merging different H cells', async () => {
  // Given
  const f = markerFixture()
  const a = await f.prepare('ordered-a')
  const b = await f.prepare('ordered-b')
  // When
  const again = await f.prepare('ordered-a')
  // Then
  expect(again.source).toBe(a.source)
  expect(b.source).toContain('ordered-b')
  expect(b.source).not.toBe(a.source)
  expect(f.count()).toBe(2)
})

it('proves native named-plugin marker collision before refusing the incompatible local private cell', async () => {
  // Given
  const f = markerFixture()
  const other = join(f.root, 'other.cjs')
  writeFileSync(
    other,
    "module.exports = function() { return function(tree) { tree.children[0].children[0].value = 'other' } }",
  )
  const a = await f.native('same-h')
  const b = await f.native('same-h', other)
  expect(b).toEqual(a)
  await f.prepare('same-h')
  // When
  const refused = await f
    .prepare('same-h', other)
    .catch((error: unknown) => error)
  // Then
  expect(refused).toBeInstanceOf(MdxCompileError)
  expect(refused).toMatchObject({
    filename: f.filename,
    cause: { option: 'loaders.options.remarkPlugins[0][0]' },
  })
  const fresh = await f.native('new-h', other)
  expect(fresh.source).toContain('other')
  expect(fresh.source).not.toBe(a.source)
})

it('retains the marker slot under actual local E and separately keys actual C and distinct E', async () => {
  // Given
  const f = markerFixture()
  const entry = createRequire(f.loader.loader).resolve('@mdx-js/loader')
  const module = await import(
    pathToFileURL(join(dirname(entry), 'lib/index.js')).href
  )
  const directory = join(f.root, 'independent')
  mkdirSync(directory)
  symlinkSync(
    resolve(dirname(entry), '../..'),
    join(directory, 'node_modules'),
    'junction',
  )
  const independentFile = join(directory, 'index.mjs')
  writeFileSync(
    independentFile,
    readFileSync(join(dirname(entry), 'lib/index.js')),
  )
  const independent: unknown = await import(pathToFileURL(independentFile).href)
  if (!isMdxRecord(independent) || typeof independent.loader !== 'function')
    throw new TypeError('missing independent installed E')
  const original = independent.loader
  await new Promise<void>((done, reject) =>
    original.call(
      {
        getOptions: () => ({ format: 'mdx' }),
        context: f.root,
        resourcePath: f.filename,
        sourceMap: true,
      },
      '# independent',
      (error: unknown) => (error ? reject(error) : done()),
    ),
  )
  const rendererError = await new Promise<unknown>((done) =>
    original.call(
      {
        getOptions: () => ({ renderer: 'unsupported' }),
        context: f.root,
        resourcePath: f.filename,
        sourceMap: true,
      },
      '# independent',
      done,
    ),
  )
  expect(rendererError).toBeInstanceOf(Error)
  expect(rendererError).toHaveProperty(
    'message',
    '`options.renderer` is no longer supported. Please see <https://mdxjs.com/migrating/v2/> for more information',
  )
  const parseError = await new Promise<unknown>((done) =>
    original.call(
      {
        getOptions: () => ({ format: 'mdx' }),
        context: f.root,
        resourcePath: f.filename,
        sourceMap: true,
      },
      '# broken\n\n<',
      done,
    ),
  )
  expect(parseError).toBeInstanceOf(Error)
  expect(parseError).toMatchObject({
    line: 3,
    column: 2,
    message: expect.stringContaining('page.mdx:3:2:'),
  })
  const effective = {
    options: { format: 'mdx' },
    sourceMap: true,
    mode: undefined,
  }
  const a = admitMdxNativeCell(module, null, effective)
  // When
  const again = admitMdxNativeCell(module, null, effective)
  const compiler = admitMdxNativeCell(
    module,
    nativePreparationContext(f.root).compiler,
    effective,
  )
  const distinct = admitMdxNativeCell(independent, null, effective)
  // Then
  expect(again.compiler).toBe(a.compiler)
  expect(compiler.compiler).not.toBe(a.compiler)
  expect(distinct.compiler).not.toBe(a.compiler)
  expect(() =>
    admitMdxNativeCell(module, null, { ...effective, sourceMap: false }),
  ).toThrow()
  expect(a.current()).toBe(true)
  effective.options.format = 'md'
  expect(a.current()).toBe(false)
})
