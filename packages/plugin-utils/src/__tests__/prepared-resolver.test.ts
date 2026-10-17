import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { createModuleResolver } from '../import-graph'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it.each(['.mdx', '.md', '.mdown'])(
  'rejects unprepared %s at the importer',
  (extension) => {
    const module = file(`src/value${extension}`, '# Raw')
    const resolver = createModuleResolver({ cwd: root, includeMdx: ['.mdown'] })
    expect(() => resolver(module, 'src/main.tsx')).toThrow(
      `${resolve(root, 'src/main.tsx')}:1:1`,
    )
    expect(() => resolver(module, 'src/main.tsx')).toThrow(module)
  },
)

it.each([
  Promise.resolve(''),
  {
    code: '',
    then() {
      throw new Error('must not invoke')
    },
  },
  Object.assign(() => {}, { code: '', then() {} }),
])('rejects a thenable without awaiting or invoking it', (value) => {
  const module = file('src/value.mdx')
  const resolver = createModuleResolver({
    cwd: root,
    prepareSource: () => value,
  })
  expect(() => resolver(module, 'src/main.tsx')).toThrow(
    'prepare Markdown before extraction',
  )
})

it('preserves a hook exception and the real aliased module before toId', () => {
  const module = file('src/value.mdx')
  const cause = new Error('compiler failed')
  const resolver = createModuleResolver({
    cwd: root,
    alias: { value: module },
    toId: () => {
      throw new Error('too early')
    },
    prepareSource: (filename) => {
      expect(filename).toBe(module)
      throw cause
    },
  })
  try {
    resolver('value', 'src/main.tsx')
  } catch (error) {
    if (!(error instanceof Error)) throw error
    expect(error.cause).toBe(cause)
    expect(error.message).toContain(module)
    return
  }
  throw new Error('Expected preparation failure')
})

it('accepts empty preparation and reads ordinary undefined sources', () => {
  const module = file('src/value.mdx', '# Raw')
  const ordinary = file('src/plain.ts', 'export const value = 1')
  const resolver = createModuleResolver({
    cwd: root,
    prepareSource: (filename) => (filename === module ? '' : undefined),
  })
  expect(resolver(module, 'main.ts')).toEqual({ path: module, code: '' })
  expect(resolver(ordinary, 'main.ts')?.code).toBe('export const value = 1')
  expect(resolver('./missing', 'main.ts')).toBeUndefined()
})

it('remaps every exact id occurrence once, leaving other filenames alone', () => {
  const module = file('src/value.mdx')
  const other = file('src/other.mdx')
  const id = 'src/[value]+(x).mdx'
  const map = { version: 3, sources: [id], mappings: 'AAKA', names: [] }
  const resolver = createModuleResolver({
    cwd: root,
    toId: (filename) => (filename === module ? id : 'other.mdx'),
    prepareSource: (filename) => ({
      code: '',
      map: filename === module ? map : undefined,
    }),
  })
  resolver(module, 'main.ts')
  resolver(other, 'main.ts')
  const original = new Error(
    `${id}:1:1: bad\nsee ${id}:1:1 and other.mdx:2:3: bad\nprefix-${id}:1:1: untouched\nother.ts:1:1: untouched`,
  )
  const error = resolver.remapError(original)
  expect(error.message).toBe(
    `${id}:6:1: bad\nsee ${id}:6:1 and ${other}:2:3 (in compiled output): bad\nprefix-${id}:1:1: untouched\nother.ts:1:1: untouched`,
  )
  expect(error.cause).toBe(original)
  expect(resolver.remapError(error)).toBe(error)
  expect(createModuleResolver({ cwd: root }).remapError(original).message).toBe(
    original.message,
  )
})

it('replaces maps for the same id and deletes a no-longer-prepared generation', () => {
  const module = file('src/value.ts')
  let generation: { code: string; map?: unknown } | undefined = {
    code: '',
    map: { version: 3, sources: [module], mappings: 'AAKA' },
  }
  const resolver = createModuleResolver({
    cwd: root,
    prepareSource: () => generation,
  })
  resolver(module, 'main.ts')
  expect(resolver.remapError(`${module}:1:1: bad`).message).toBe(
    `${module}:6:1: bad`,
  )
  generation = {
    code: '',
    map: { version: 3, sources: [module], mappings: 'AAQA' },
  }
  resolver(module, 'main.ts')
  expect(resolver.remapError(`${module}:1:1: bad`).message).toBe(
    `${module}:9:1: bad`,
  )
  generation = undefined
  resolver(module, 'main.ts')
  expect(resolver.remapError(`${module}:1:1: bad`).message).toBe(
    `${module}:1:1: bad`,
  )
})

it('discards stale Markdown mappings when preparation disappears', () => {
  const module = file('src/value.mdx')
  let prepared: string | undefined = ''
  const resolver = createModuleResolver({
    cwd: root,
    prepareSource: () => prepared,
  })
  resolver(module, 'main.ts')
  prepared = undefined
  expect(() => resolver(module, 'main.ts')).toThrow(module)
  expect(resolver.remapError(`${module}:1:1: bad`).message).toBe(
    `${module}:1:1: bad`,
  )
})

it('keeps exact Windows ids, uncovered positions and non-Error hook causes', () => {
  const module = file('src/value.mdx')
  const id = 'C:\\folder\\[value]+.mdx'
  const resolver = createModuleResolver({
    cwd: root,
    toId: () => id,
    prepareSource: () => ({
      code: '',
      map: { version: 3, sources: [module], mappings: 'AAAA' },
    }),
  })
  resolver(module, 'main.ts')
  expect(
    resolver.remapError(`${id}:2:1: bad\nlong-${id}:1:1: other`).message,
  ).toBe(`${module}:2:1 (in compiled output): bad\nlong-${id}:1:1: other`)
  const failed = createModuleResolver({
    cwd: root,
    prepareSource: () => {
      throw { toString: () => 'compiler not ready' }
    },
  })
  expect(() => failed(module, 'main.ts')).toThrow('compiler not ready')
})
