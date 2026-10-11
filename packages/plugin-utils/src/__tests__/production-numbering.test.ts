import { afterEach, beforeEach, expect, it } from 'bun:test'

import * as wasm from '../../../../bindings/devup-ui-wasm/pkg'
import {
  BuildGeneration,
  ClosedBuildGenerationError,
} from '../build-generation'
import {
  type ProductionExtractionInput,
  ProductionNumbering,
  ProductionNumberingError,
} from '../production-numbering'

const file = {
  context: 'A',
  id: 'a.tsx',
  path: '/alias/a.tsx',
  realPath: '/physical/a.tsx',
}
const plan = {
  context: 'A',
  files: [file, file, { ...file, id: 'toString' }],
  nonphysical: [],
  canonical: { 'a.tsx': 'bucket', missing: 'bucket' },
}
const input: ProductionExtractionInput = {
  context: 'A',
  id: 'a.tsx',
  source: { kind: 'physical', path: file.path, realPath: file.realPath },
  location: { filename: file.path, importer: '/entry.tsx' },
}
let owner: BuildGeneration<unknown>
let numbering: ProductionNumbering
let debug: boolean
beforeEach(() => {
  debug = wasm.isDebug()
  wasm.resetBuildState()
  wasm.setDebug(false)
  owner = new BuildGeneration()
  numbering = new ProductionNumbering(owner, [plan])
})
afterEach(() => {
  wasm.resetBuildState()
  wasm.setModuleResolver(undefined)
  wasm.registerTheme({})
  wasm.registerShorthands({})
  wasm.setDebug(debug)
})
function failure(action: () => unknown): ProductionNumberingError {
  try {
    action()
  } catch (error) {
    if (error instanceof ProductionNumberingError) return error
    throw error
  }
  throw new Error('Expected production numbering failure')
}

it('retains deep copies when caller-owned plans mutate', () => {
  // Given mutable records, arrays and canonical configuration.
  const record = { ...file }
  const canonical = { 'a.tsx': 'bucket', virtual: 'virtual-bucket' }
  const files = [record]
  const nonphysical = ['virtual']
  const plans = [{ context: 'A', files, canonical, nonphysical }]
  const frozen = new ProductionNumbering(owner, plans)
  record.path = '/changed'
  canonical['a.tsx'] = 'changed'
  canonical.virtual = 'changed-virtual'
  nonphysical[0] = 'changed-original'
  nonphysical.push('late')
  files.length = 0
  plans.length = 0
  // When seed uses the constructor's authority.
  frozen.seed(wasm, 'A')
  // Then copies preserve exact membership and canonical configuration.
  expect(frozen.files).toEqual([file])
  expect(frozen.buckets).toEqual([
    'a.tsx',
    'bucket',
    'virtual',
    'virtual-bucket',
  ])
  expect(
    [frozen.files, ...frozen.files, frozen.buckets].every(Object.isFrozen),
  ).toBe(true)
  expect(wasm.exportFileMap()).toBe(
    '{"a.tsx":0,"bucket":1,"virtual":2,"virtual-bucket":3}',
  )
  expect(JSON.parse(wasm.exportCanonicalMap())).toEqual({
    'a.tsx': 'bucket',
    virtual: 'virtual-bucket',
  })
  expect(() => frozen.assertReserved(input)).not.toThrow()
})

it.each([
  { context: 'other' },
  { id: 'bucket' },
  { id: 'missing' },
  { source: { ...input.source, path: file.realPath } },
  { source: { ...input.source, realPath: '/elsewhere' } },
])('rejects exact provenance misses when input changes by %j', (change) => {
  // Given a seeded reservation whose bucket alone is not membership.
  numbering.seed(wasm, 'A')
  const changed = { ...input, ...change }
  // When exact physical membership fails.
  const error = failure(() => numbering.assertReserved(changed))
  // Then the typed discovery defect retains the actual boundary metadata.
  expect(error.reason).toBe('physical-miss')
  expect([error.context, error.id]).toEqual([changed.context, changed.id])
  expect(error.location).toEqual({ ...input.location, line: 1, column: 1 })
  expect(Object.isFrozen(error.location)).toBe(true)
  expect(Object.hasOwn(error, 'cause')).toBe(false)
})

it('retains supplied coordinates when extraction is unseeded', () => {
  // Given an unseeded primitive and a known source span.
  const location = { filename: file.path, line: 7, column: 9 }
  // When physical admission precedes seed.
  const error = failure(() => numbering.assertReserved({ ...input, location }))
  // Then the known span is not replaced by boundary defaults.
  expect(error.reason).toBe('unseeded')
  expect(error.location).toEqual(location)
  expect(wasm.exportFileMap()).toBe('{}')
})

it.each([
  { plans: [plan, plan] },
  { plans: [{ ...plan, files: [{ ...file, context: 'B' }] }] },
  { plans: [{ ...plan, files: [file, { ...file, path: '/other' }] }] },
  { plans: [{ ...plan, files: [file, { ...file, realPath: '/other' }] }] },
])('rejects invalid authority when plans are %j', ({ plans }) => {
  // Given conflicting contexts or original-ID provenance.
  // When construction validates the complete plans.
  const error = failure(() => new ProductionNumbering(owner, plans))
  // Then construction fails before allocation.
  expect(error.reason).toBe('invalid-plan')
  expect(error.context).toBe('A')
  expect(wasm.exportFileMap()).toBe('{}')
})

it('leaves configuration untouched when active context is unknown', () => {
  // Given an independently configured real engine.
  wasm.importCanonicalMap({ prior: 'target' })
  // When context selection has no frozen authority.
  const error = failure(() => numbering.seed(wasm, 'B'))
  // Then no identity install or allocation occurred.
  expect(error.reason).toBe('invalid-plan')
  expect(JSON.parse(wasm.exportCanonicalMap())).toEqual({ prior: 'target' })
  expect(wasm.exportFileMap()).toBe('{}')
})

it.each([false, true])(
  'preserves caller replay when successful seed repeats, empty=%s',
  (empty) => {
    // Given a successfully seeded real engine, including prototype-shadowing IDs.
    const selected = empty ? { ...plan, files: [] } : plan
    const once = new ProductionNumbering(owner, [selected])
    once.seed(wasm, 'A')
    const expected = empty ? '{}' : '{"a.tsx":0,"bucket":1,"toString":2}'
    wasm.importCanonicalMap({ replay: 'independent' })
    // When seed repeats successfully.
    once.seed(wasm, 'A')
    // Then the entire file map and current replayed configuration survive unchanged.
    expect(wasm.exportFileMap()).toBe(expected)
    expect(JSON.parse(wasm.exportCanonicalMap())).toEqual({
      replay: 'independent',
    })
  },
)

it.each([
  'identity',
  'seed',
  'restore',
  'dual',
  'identity-dual',
  'undefined',
  'empty',
])('retains terminal causes when %s fails', (mode) => {
  // Given narrow unavailable-fault delegates around the real engine.
  const selected = mode === 'empty' ? { ...plan, files: [] } : plan
  const terminal = new ProductionNumbering(owner, [selected])
  const cause = mode === 'undefined' ? undefined : new RangeError(mode)
  const restoreCause = new TypeError('restore fault')
  const engine = {
    seedFileMap(files: string[]) {
      if (['seed', 'dual', 'undefined', 'empty'].includes(mode)) throw cause
      wasm.seedFileMap(files)
    },
    importCanonicalMap(map: Readonly<Record<string, string>>) {
      if (Object.keys(map).length === 0 && mode.startsWith('identity'))
        throw cause
      if (
        Object.keys(map).length > 0 &&
        ['restore', 'dual', 'identity-dual'].includes(mode)
      )
        throw restoreCause
      wasm.importCanonicalMap(map)
    },
  }
  // When identity, seed (even empty) or restoration fails.
  const error = failure(() => terminal.seed(engine, 'A'))
  // Then finally cannot mask the primary fault and no terminal call retries.
  expect(error.reason).toBe('seed-failure')
  expect(error.cause).toBe(mode === 'restore' ? restoreCause : cause)
  expect(error.restoreCause).toBe(
    ['restore', 'dual', 'identity-dual'].includes(mode)
      ? restoreCause
      : undefined,
  )
  expect(Object.hasOwn(error, 'cause')).toBe(true)
  expect(failure(() => terminal.seed(wasm, 'A'))).toBe(error)
  expect(failure(() => terminal.assertReserved(input))).toBe(error)
})

it('rejects all boundaries when the actual owner disposed', () => {
  // Given a retained primitive and a closed owner.
  owner.dispose()
  // When constructor, seed and physical admission cross the disposed boundary.
  // Then the existing owner error owns each rejection, not a new numbering reason.
  expect(() => new ProductionNumbering(owner, [plan])).toThrow(
    ClosedBuildGenerationError,
  )
  expect(() => numbering.seed(wasm, 'A')).toThrow(ClosedBuildGenerationError)
  expect(() => numbering.assertReserved(input)).toThrow(
    ClosedBuildGenerationError,
  )
})
