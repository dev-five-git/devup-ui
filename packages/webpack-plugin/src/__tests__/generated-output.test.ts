import { writeFileSync } from 'node:fs'
import * as disk from 'node:fs/promises'
import { mkdtemp, readFile, rm, stat, utimes } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { afterEach, beforeEach, expect, it, mock, spyOn } from 'bun:test'

import { bindCompilerScope } from '../build-scope'
import {
  createGeneratedOutputWriter,
  normalizeSheet,
} from '../generated-output'
import loader from '../loader'

let root: string
let scope: ReturnType<typeof bindCompilerScope>
const compiler = { root: {} }
const property = { c: 'a', p: 'color', v: 'red', s: null }
const sheet = (reversed = false) =>
  JSON.stringify({
    properties: {
      'entry.tsx': {
        0: {
          0: reversed
            ? [{ ...property, h: true }, property]
            : [property, { ...property, h: true }],
        },
      },
    },
    imports: { 'entry.tsx': ['a', 'b'] },
    font_faces: { 'entry.tsx': [{ src: 'a' }, { src: 'b' }] },
    layer_orders: { 'entry.tsx': ['a', 'b'] },
    keyframes: {
      'entry.tsx': {
        spin: {
          from: [
            ['color', 'red'],
            ['color', 'blue'],
          ],
        },
      },
    },
    css: {
      'entry.tsx': [
        {
          css: '@media a{@supports b{}}',
          atRules: ['@media a', '@supports b'],
        },
      ],
    },
    atomNamingVersion: 2,
  })
async function extract() {
  return new Promise<void>((done, reject) =>
    Reflect.apply(
      loader,
      {
        _compiler: compiler,
        resourcePath: join(root, 'entry.tsx'),
        getOptions: () => ({
          package: '@devup-ui/react',
          cssDir: root,
          rootDir: root,
          watch: true,
          singleCss: true,
          sheetFile: join(root, 'sheet.json'),
          classMapFile: join(root, 'class.json'),
          fileMapFile: join(root, 'file.json'),
        }),
        async: () => (error: Error | null) => (error ? reject(error) : done()),
        addDependency() {},
        addMissingDependency() {},
      },
      [
        Buffer.from(
          'import {Box} from "@devup-ui/react";export const x=<Box color="red"/>',
        ),
      ],
    ),
  )
}
beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), 'devup-publication-'))
  scope = bindCompilerScope(compiler)
  scope.setConfiguration(() => {
    wasm.setPrefix(null)
    wasm.registerShorthands({})
    wasm.registerTheme({})
  })
})
afterEach(async () => {
  mock.restore()
  scope.close()
  await rm(root, { recursive: true, force: true })
})

it('preserves sheet bytes, mtime and writes when only property-set order changes', async () => {
  // Given: the real loader has completed publication to real files.
  const exported = spyOn(wasm, 'exportSheet').mockReturnValue(sheet())
  await extract()
  const path = join(root, 'sheet.json')
  const bytes = await readFile(path, 'utf-8')
  await utimes(path, 1, 1)
  const writes = spyOn(disk, 'writeFile')
  exported.mockReturnValue(sheet(true))
  // When: the same set is serialized in the opposite order.
  await extract()
  // Then: neither content nor filesystem publication changes.
  expect(await readFile(path, 'utf-8')).toBe(bytes)
  expect((await stat(path)).mtimeMs).toBe(1000)
  expect(writes.mock.calls.filter(([file]) => file === path)).toHaveLength(0)
})

it.each(['sheet', 'class', 'file'])(
  'publishes meaningful %s metadata changes',
  async (kind) => {
    // Given
    const exported = spyOn(
      wasm,
      kind === 'sheet'
        ? 'exportSheet'
        : kind === 'class'
          ? 'exportClassMap'
          : 'exportFileMap',
    )
    exported.mockReturnValue(kind === 'sheet' ? sheet() : '{}')
    await extract()
    const path = join(root, `${kind}.json`)
    const before = await readFile(path, 'utf-8')
    // When
    exported.mockReturnValue(
      kind === 'sheet'
        ? sheet().replace('red', 'green')
        : kind === 'class'
          ? '{"changed":{}}'
          : '{"changed":1}',
    )
    await extract()
    // Then
    expect(await readFile(path, 'utf-8')).not.toBe(before)
  },
)

it.each(['imports', 'font_faces', 'layer_orders', 'keyframes', 'css'])(
  'retains ordered %s arrays and every serialized property field',
  async (field) => {
    // Given
    const initial = normalizeSheet(sheet())
    const changed = JSON.parse(sheet())
    const bucket = changed[field]['entry.tsx']
    if (field === 'keyframes') bucket.spin.from.reverse()
    else if (field === 'css') bucket[0].atRules.reverse()
    else bucket.reverse()
    const writer = createGeneratedOutputWriter()
    const path = join(root, 'ordered.json')
    await writer.write(path, initial)
    // When
    await writer.write(path, normalizeSheet(JSON.stringify(changed)))
    // Then
    expect(await readFile(path, 'utf-8')).not.toBe(initial)
    expect(JSON.parse(initial).properties['entry.tsx'][0][0]).toContainEqual({
      ...property,
      h: true,
    })
  },
)

it('reads inside the queue for persisted A, in-flight B, pending C then A; other paths remain independent', async () => {
  // Given
  const writer = createGeneratedOutputWriter()
  const path = join(root, 'queue')
  await writer.write(path, 'A', 'utf-8')
  const started = Promise.withResolvers<void>()
  const release = Promise.withResolvers<void>()
  const writes = spyOn(disk, 'writeFile').mockImplementation(
    async (file, content) => {
      if (typeof file !== 'string' || typeof content !== 'string')
        throw new TypeError('Expected string publication inputs')
      if (content === 'B') {
        started.resolve()
        await release.promise
      }
      writeFileSync(file, content)
    },
  )
  const b = writer.write(path, 'B')
  await started.promise
  const c = writer.write(path, 'C')
  const a = writer.write(path, 'A')
  await writer.write(join(root, 'other'), 'independent')
  // When
  release.resolve()
  await Promise.all([b, c, a])
  // Then
  expect(await readFile(path, 'utf-8')).toBe('A')
  expect(writes.mock.calls.map(([, content]) => content)).toEqual([
    'B',
    'independent',
    'A',
  ])
})

it.each(['readFile', 'writeFile'] as const)(
  'holds the loader callback for required publication and retries original %s errors',
  async (operation) => {
    // Given
    await extract()
    const path = join(root, 'class.json')
    spyOn(wasm, 'exportClassMap').mockReturnValue('{"changed":{}}')
    const failure = Object.assign(new Error('disk fault'), { code: 'EACCES' })
    const gate = Promise.withResolvers<void>()
    const started = Promise.withResolvers<void>()
    const fault = spyOn(disk, operation).mockImplementationOnce(async () => {
      started.resolve()
      await gate.promise
      throw failure
    })
    let completed = false
    const pending = extract()
      .then(
        () => undefined,
        (error: unknown) => error,
      )
      .finally(() => {
        completed = true
      })
    await Promise.race([started.promise, pending])
    expect(completed).toBe(false)
    // When
    gate.resolve()
    expect(await pending).toBe(failure)
    fault.mockRestore()
    await extract()
    // Then
    expect(await readFile(path, 'utf-8')).toBe('{"changed":{}}')
  },
)
