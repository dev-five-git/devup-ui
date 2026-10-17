import { expect, it } from 'bun:test'

import { equalMdxIsolatePipeline } from '../mdx-isolate-pipeline'
import {
  parseMdxIsolateDemand,
  parseMdxIsolateOutcome,
  parseMdxIsolateVerdict,
  requireMdxIsolateData,
} from '../mdx-isolate-protocol'
import type { MdxPipeline } from '../mdx-pipeline'

it('accepts cloneable plain data with shared references without serializing opaque objects', () => {
  // Given
  const shared = {
    text: 'data',
    flag: true,
    count: 1,
    absent: undefined,
    nil: null,
  }
  const value = { first: shared, second: shared, list: [shared] }
  // When
  requireMdxIsolateData(value)
  // Then
  expect(structuredClone(value)).toEqual(value)
})

it.each([
  () => undefined,
  new AbortController().signal,
  new Map(),
  new Date(),
  Symbol('opaque'),
])('refuses non-DATA transport before any worker publication: %s', (value) => {
  // Given / When / Then
  expect(() => requireMdxIsolateData({ value })).toThrow('plain DATA only')
})

it('refuses an accessor without invoking it', () => {
  // Given
  let reads = 0
  const value = {
    get property() {
      reads++
      return 'effect'
    },
  }
  // When
  expect(() => requireMdxIsolateData(value)).toThrow(
    'cannot evaluate accessors',
  )
  // Then
  expect(reads).toBe(0)
})

const pipeline = {
  bundler: 'turbo',
  ruleKey: '*.mdx',
  conditions: [],
  aliases: {},
  loaders: [{ loader: 'compiler' }],
}
const demand = { run: 'one', filename: '/page.mdx', timeoutMs: 100, pipeline }
const output = {
  filename: '/page.mdx',
  source: 'compiled',
  map: 'genuine-map',
  dependencies: [],
  contextDependencies: [],
  missingDependencies: [],
  buildDependencies: [],
  dependencyReports: [],
}

it.each([
  null,
  { ...demand, pipeline: { ...pipeline, bundler: 'webpack' } },
  { ...demand, pipeline: { ...pipeline, loaders: [null] } },
  {
    ...demand,
    pipeline: {
      ...pipeline,
      loaders: [{ loader: 'compiler', ident: 'unexpected' }],
    },
  },
])('rejects malformed demand data at its boundary: %s', (value) => {
  // Given / When / Then
  expect(() => parseMdxIsolateDemand(value)).toThrow(TypeError)
})

it.each([
  null,
  { kind: 'failed', error: {} },
  { kind: 'other' },
  { kind: 'prepared', output: {}, inputs: [] },
  { kind: 'prepared', output: { ...output, dependencies: null }, inputs: [] },
  {
    kind: 'prepared',
    output: { ...output, dependencyReports: [{}] },
    inputs: [],
  },
  {
    kind: 'prepared',
    output,
    inputs: [{ kind: 'file', path: '/input', loader: 'compiler' }],
  },
])('rejects malformed result data at its boundary: %s', (value) => {
  // Given / When / Then
  expect(() => parseMdxIsolateOutcome(value)).toThrow(TypeError)
})

it('compares cloned condition RegExp source/flags and ordered compiler options independently', () => {
  // Given
  const selected: MdxPipeline = {
    ...pipeline,
    bundler: 'turbo',
    conditions: [{ condition: { path: /\.mdx$/i } }],
    loaders: [{ loader: 'compiler', options: { first: 1, second: 2 } }],
  }
  const clone = structuredClone(selected)
  // When / Then
  requireMdxIsolateData(selected)
  expect(equalMdxIsolatePipeline(selected, clone)).toBe(true)
  expect(
    equalMdxIsolatePipeline(selected, {
      ...clone,
      conditions: [{ condition: { path: /\.md$/i } }],
    }),
  ).toBe(false)
  expect(
    equalMdxIsolatePipeline(selected, {
      ...clone,
      conditions: [{ condition: { path: /\.mdx$/ } }],
    }),
  ).toBe(false)
  expect(
    equalMdxIsolatePipeline(selected, {
      ...clone,
      loaders: [{ loader: 'compiler', options: { second: 2, first: 1 } }],
    }),
  ).toBe(false)
})

it.each([
  Object.assign(/x/, { custom: 'lost' }),
  new (class extends RegExp {})('x'),
  Object.defineProperty(/x/, 'custom', { get: () => 'lost' }),
  Object.assign(/x/g, { lastIndex: 1 }),
])(
  'refuses RegExp state which structured clone would silently discard: %s',
  (value) => {
    // Given / When / Then
    expect(() => requireMdxIsolateData(value)).toThrow(
      'RegExp subclass/custom state',
    )
  },
)

it('preserves descriptor and JSON option own-key order through demand cloning', () => {
  // Given
  const descriptor = { options: { second: 2, first: 1 }, loader: 'compiler' }
  const input = { ...demand, pipeline: { ...pipeline, loaders: [descriptor] } }
  // When
  const parsed = structuredClone(parseMdxIsolateDemand(input))
  // Then
  expect(Object.keys(parsed.pipeline.loaders[0] ?? {})).toEqual([
    'options',
    'loader',
  ])
  expect(Object.keys(parsed.pipeline.loaders[0]?.options ?? {})).toEqual([
    'second',
    'first',
  ])
})

it('refuses a verdict carrying another resource error', () => {
  // Given
  const verdict = {
    run: 'one',
    filename: '/one.mdx',
    error: {
      filename: '/other.mdx',
      message: 'wrong resource',
      line: 1,
      column: 1,
    },
  }
  // When / Then
  expect(() => parseMdxIsolateVerdict(verdict)).toThrow(TypeError)
})
