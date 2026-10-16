import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { isMdxRecord } from '../mdx-pipeline'
import { markerFixture } from './mdx-marker-fixture'
import { turboWorkerFixture } from './mdx-turbo-worker-fixture'
import observe, {
  pitch,
  privateCompile,
  selectedOptions,
  wireOptions,
} from './mdx-turbo-worker-observer'

it.each([undefined, false, true])(
  'executes the observer source against the installed chain as a coverage counterpart isolation=%s',
  async (privateFirst) => {
    // Given: real-worker evidence lives in mdx-turbo-worker.test.ts, not this local counterpart.
    const f = markerFixture()
    const alternate = join(f.root, 'alternate.cjs')
    writeFileSync(
      alternate,
      'module.exports = function() { return function(tree) { tree.children[0].children[0].value = "private-only" } }',
    )
    const receiving = {
      ...f.receiving,
      resourcePath: f.filename,
      context: dirname(f.filename),
      loaders: [
        { path: 'tap', options: {} },
        { path: f.loader.loader, options: f.options('counterpart') },
      ],
      data: {},
      getOptions: () =>
        privateFirst === undefined ? {} : { alternate, privateFirst },
      async: () => (error?: unknown) => {
        if (error) throw error
      },
    }
    if (privateFirst)
      await new Promise<void>((done, reject) =>
        pitch.call({
          ...receiving,
          async: () => (error?: unknown) => (error ? reject(error) : done()),
        }),
      )
    else pitch.call(receiving)
    const native = await f.native('counterpart')
    // When
    await new Promise<void>((done, reject) =>
      observe.call(
        {
          ...receiving,
          async: () => (error?: unknown) => (error ? reject(error) : done()),
        },
        native.source,
        native.map,
      ),
    )
    // Then
    const record: unknown = JSON.parse(
      readFileSync(join(f.root, 'observations.jsonl'), 'utf8'),
    )
    if (!isMdxRecord(record))
      throw new TypeError('missing counterpart observation')
    expect(record.repeated).toEqual(native)
    if (privateFirst === undefined) expect(record.prepared).toEqual(native)
    else
      expect(record.prepared).toMatchObject({
        source: expect.stringContaining('private-only'),
      })
  },
)

it('propagates installed compiler failures through the observer callback and private error record', async () => {
  // Given
  const f = markerFixture()
  const receiving = {
    ...f.receiving,
    resourcePath: f.filename,
    context: dirname(f.filename),
    loaders: [
      { path: 'tap', options: {} },
      { path: f.loader.loader, options: { renderer: 'unsupported' } },
    ],
    data: {},
    getOptions: () => ({}),
    async: () => () => undefined,
  }
  pitch.call(receiving)
  // When
  const prepared = await privateCompile(receiving, { renderer: 'unsupported' })
  const error = await new Promise<unknown>((done) =>
    observe.call({ ...receiving, async: () => done }, 'native', undefined),
  )
  // Then
  expect(prepared).toHaveProperty('error')
  expect(error).toBeInstanceOf(Error)
})

it('refuses incomplete observation slots and wire snapshots before executing a compiler', async () => {
  // Given
  const f = markerFixture()
  const receiving = {
    ...f.receiving,
    resourcePath: f.filename,
    loaders: [],
    data: {},
    getOptions: () => ({}),
    async: () => () => undefined,
  }
  // When / Then
  expect(() => pitch.call(receiving)).toThrow(TypeError)
  expect(() => wireOptions(receiving)).toThrow(TypeError)
  expect(() => wireOptions({ ...receiving, data: { wire: 'null' } })).toThrow(
    TypeError,
  )
  expect(() => selectedOptions({}, 'alternate.cjs')).toThrow(TypeError)
  expect(privateCompile(receiving, {})).rejects.toBeInstanceOf(TypeError)
  const error = await new Promise<unknown>((done) =>
    observe.call({ ...receiving, async: () => done }, 'native', undefined),
  )
  expect(error).toBeInstanceOf(TypeError)
})

it('retains native parse errors and cleans up a failed observer build', async () => {
  // Given
  const f = markerFixture()
  writeFileSync(f.filename, '# broken\n\n<')
  // When
  const native = await f.native('unused').catch((error: unknown) => error)
  const build = await turboWorkerFixture(
    join(f.root, 'missing-observer.ts'),
  ).catch((error: unknown) => error)
  // Then
  expect(native).toBeInstanceOf(Error)
  expect(build).toBeInstanceOf(Error)
  expect(native).toMatchObject({
    line: 3,
    column: 2,
    message: expect.stringContaining('page.mdx:3:2:'),
  })
  writeFileSync(f.filename, '# corrected')
  const recovery = await f.native('unused')
  expect(recovery.source).toContain('unused')
  expect(recovery.source).not.toContain('corrected')
  expect(f.count()).toBe(1)
})
