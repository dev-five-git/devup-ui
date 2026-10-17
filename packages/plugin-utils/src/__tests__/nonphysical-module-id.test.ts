import { expect, it } from 'bun:test'

import {
  type NonphysicalModuleId,
  normalizeNonphysicalModuleId,
  type PhysicalImporter,
} from '../nonphysical-module-id'

it.each([
  '\0virtual:entry.tsx?raw#fragment',
  'virtual:entry.tsx?one#first',
  'virtual:entry.tsx?two#second',
  'virtual:entry.tsx#first?one',
  'virtual:entry.tsx#second?one',
  '.\\native\\..\\entry.tsx',
  './native/../entry.tsx',
  'é/e\u0301/😀/文件.tsx',
  '  opaque  ',
])('preserves native identity %j over a different request', (id) => {
  // Given a native identity distinct from the importer-relative fallback.
  const resolved = { namespace: 'native:producer', id }
  const request = { namespace: 'written', specifier: './other.tsx' }
  const importer: PhysicalImporter = {
    path: '/real/src/main.tsx',
    pathStyle: 'posix',
  }
  // When native resolution supplies the identity.
  const result = normalizeNonphysicalModuleId(resolved, request, importer)
  // Then every opaque byte and the native namespace survive in a frozen copy.
  expect(result).toEqual(resolved)
  expect(result).not.toBe(resolved)
  expect(Object.isFrozen(result)).toBe(true)
})

interface RelativeCase {
  readonly importer: PhysicalImporter
  readonly specifier: string
  readonly expected: string
}
const relatives: readonly RelativeCase[] = [
  {
    importer: { path: '/real/src/main.tsx', pathStyle: 'posix' },
    specifier: './a/../child',
    expected: '/real/src/child',
  },
  {
    importer: { path: '/real/src/main.tsx', pathStyle: 'posix' },
    specifier: '../child?raw#x?again',
    expected: '/real/child?raw#x?again',
  },
  {
    importer: { path: '/real/src/main.tsx', pathStyle: 'posix' },
    specifier: './child#x?raw',
    expected: '/real/src/child#x?raw',
  },
  {
    importer: { path: '/real/src/main.tsx', pathStyle: 'posix' },
    specifier: './back\\slash?x\\y',
    expected: '/real/src/back\\slash?x\\y',
  },
  {
    importer: { path: '/real/src/main.tsx', pathStyle: 'posix' },
    specifier: './é%2Fchild',
    expected: '/real/src/é%2Fchild',
  },
  {
    importer: { path: 'C:\\real\\src\\main.tsx', pathStyle: 'win32' },
    specifier: '.\\a\\..\\child',
    expected: 'C:\\real\\src\\child',
  },
  {
    importer: { path: 'C:\\real\\src\\main.tsx', pathStyle: 'win32' },
    specifier: '..\\child?raw#x/../y',
    expected: 'C:\\real\\child?raw#x/../y',
  },
  {
    importer: { path: 'C:\\real\\src\\main.tsx', pathStyle: 'win32' },
    specifier: './child#x?raw',
    expected: 'C:\\real\\src\\child#x?raw',
  },
  {
    importer: { path: 'C:\\real\\src\\main.tsx', pathStyle: 'win32' },
    specifier: '../child',
    expected: 'C:\\real\\child',
  },
  {
    importer: { path: '\\\\server\\share\\src\\main.tsx', pathStyle: 'win32' },
    specifier: '.\\child?😀',
    expected: '\\\\server\\share\\src\\child?😀',
  },
]
it.each([...relatives])(
  'resolves only the relative path portion of $specifier',
  ({ importer, specifier, expected }) => {
    // Given caller-decoded nonphysical input and an actual physical importer.
    const request = { namespace: 'fallback', specifier }
    // When no native resolved identity exists.
    const result = normalizeNonphysicalModuleId(undefined, request, importer)
    // Then explicit path style controls the path and suffix bytes remain exact.
    expect(result).toEqual({ namespace: 'fallback', id: expected })
    expect(Object.isFrozen(result)).toBe(true)
    expect(request.specifier).toBe(specifier)
  },
)

it.each([
  'bare?raw#x',
  'virtual:./entry',
  'file:../entry',
  '/absolute/../entry',
  'C:\\absolute\\entry',
  '\\\\server\\share\\entry',
  '\0./entry',
  './entry?\0raw',
  '../entry#\0fragment',
  '.',
  '..',
])('retains opaque fallback %j under either path style', (specifier) => {
  // Given an opaque request with a genuine importer for each explicit style.
  const importers: readonly PhysicalImporter[] = [
    { path: '/real/main.tsx', pathStyle: 'posix' },
    { path: 'C:\\real\\main.tsx', pathStyle: 'win32' },
  ]
  // When fallback identity is selected for each caller context.
  const results = importers.map((importer) =>
    normalizeNonphysicalModuleId(
      undefined,
      { namespace: null, specifier },
      importer,
    ),
  )
  // Then requests are not reclassified or normalized as physical paths.
  expect(results).toEqual(
    importers.map(() => ({ namespace: null, id: specifier })),
  )
})

it.each(['./child?raw#x', '../child', '.\\child', '..\\child', '\0virtual:x'])(
  'retains %j without an actual physical importer',
  (specifier) => {
    // Given caller-established nonphysical identity without importer provenance.
    const request = { namespace: '', specifier }
    // When no native identity or physical importer exists.
    const result = normalizeNonphysicalModuleId(undefined, request, undefined)
    // Then no host cwd, separator inference or extension is introduced.
    expect(result).toEqual({ namespace: '', id: specifier })
    expect(Object.isFrozen(result)).toBe(true)
  },
)

it('keeps Windows-relative spelling opaque under POSIX style', () => {
  // Given backslashes are not POSIX path separators.
  const request = { namespace: null, specifier: '.\\child' }
  // When a POSIX physical importer is supplied.
  const result = normalizeNonphysicalModuleId(undefined, request, {
    path: '/real/main.tsx',
    pathStyle: 'posix',
  })
  // Then the spelling survives without host-platform inference.
  expect(result).toEqual({ namespace: null, id: '.\\child' })
})

it('uses decoded request evidence without decoding opaque percent bytes again', () => {
  // Given written escape bytes and a separately decoded specifier.
  const occurrence = {
    written: String.raw`./\x61%2Fchild`,
    specifier: './a%2Fchild',
  }
  // When the caller supplies its decoded specifier, not written request bytes.
  const result = normalizeNonphysicalModuleId(
    undefined,
    { namespace: null, specifier: occurrence.specifier },
    { path: '/real/main.tsx', pathStyle: 'posix' },
  )
  // Then it neither decodes twice nor invents a parser filename extension.
  expect(result).toEqual({ namespace: null, id: '/real/a%2Fchild' })
})

it('keeps namespace boundaries distinct even when colon flattening would collide', () => {
  // Given null, empty, named and ambiguous colon-containing pairs.
  const identities: readonly NonphysicalModuleId[] = [
    { namespace: null, id: 'same' },
    { namespace: '', id: 'same' },
    { namespace: 'one', id: 'same' },
    { namespace: 'two', id: 'same' },
    { namespace: 'a:b', id: 'c' },
    { namespace: 'a', id: 'b:c' },
  ]
  // When native identities are copied without a flattened filename encoding.
  const results = identities.map((identity) =>
    normalizeNonphysicalModuleId(
      identity,
      { namespace: 'ignored', specifier: 'ignored' },
      undefined,
    ),
  )
  // Then structured identities remain lossless and all six differ.
  expect(results).toEqual([...identities])
  expect(
    new Set(results.map(({ namespace, id }) => JSON.stringify([namespace, id])))
      .size,
  ).toBe(6)
})

it('detaches native copies from later input mutation without mutating inputs', () => {
  // Given independently mutable caller fixtures.
  const resolved = { namespace: 'native', id: '\0opaque?raw#x' }
  const request = { namespace: 'request', specifier: './other' }
  const importer: PhysicalImporter = {
    path: '/real/main.tsx',
    pathStyle: 'posix',
  }
  // When normalization returns a native copy.
  const result = normalizeNonphysicalModuleId(resolved, request, importer)
  // Then caller inputs are unchanged and later native mutation cannot alter it.
  expect(resolved).toEqual({ namespace: 'native', id: '\0opaque?raw#x' })
  expect(request).toEqual({ namespace: 'request', specifier: './other' })
  expect(importer).toEqual({ path: '/real/main.tsx', pathStyle: 'posix' })
  resolved.namespace = 'changed'
  resolved.id = 'changed'
  request.specifier = 'changed'
  expect(result).toEqual({ namespace: 'native', id: '\0opaque?raw#x' })
})

it('detaches fallback identity from later request and importer mutation', () => {
  // Given mutable caller request and physical-importer records.
  const request = { namespace: 'fallback', specifier: './child?x#y' }
  const importer: { path: string; pathStyle: 'posix' } = {
    path: '/real/main.tsx',
    pathStyle: 'posix',
  }
  // When fallback normalization returns its frozen value.
  const result = normalizeNonphysicalModuleId(undefined, request, importer)
  // Then input records are unchanged and later changes do not reach the result.
  expect(request).toEqual({ namespace: 'fallback', specifier: './child?x#y' })
  expect(importer.path).toBe('/real/main.tsx')
  request.namespace = 'changed'
  request.specifier = 'changed'
  importer.path = '/other/main.tsx'
  expect(result).toEqual({ namespace: 'fallback', id: '/real/child?x#y' })
  expect(Object.isFrozen(result)).toBe(true)
})
