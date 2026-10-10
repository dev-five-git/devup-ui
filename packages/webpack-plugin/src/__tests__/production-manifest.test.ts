import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import { bindCompilerScope, createWebpackGeneration } from '../build-scope'
import {
  readWebpackProductionManifest as read,
  registerWebpackReturnedConfig as register,
  sealWebpackProductionManifest as seal,
  WebpackProductionManifestError,
  type WebpackProductionRole,
} from '../production-manifest'

function rejection(action: () => unknown): WebpackProductionManifestError {
  try {
    action()
  } catch (error) {
    if (error instanceof WebpackProductionManifestError) return error
    throw error
  }
  throw new Error('Expected registry rejection')
}

it.each([
  ['client', 'nodejs', 'edge'],
  ['edge', 'nodejs', 'client'],
] satisfies WebpackProductionRole[][])(
  'retains exact per-role membership when arrivals are %s/%s/%s',
  (...roles) => {
    // Given
    const owner = createWebpackGeneration()
    const first: Configuration = {}
    const second: Configuration = {}
    // When
    for (const role of roles) register(owner, role, first)
    register(owner, 'client', first)
    register(owner, 'client', second)
    const result = read(owner)
    // Then
    expect(result.phase).toBe('open')
    expect(result.configs.client).toEqual([first, second])
    expect(result.configs.client[0]).toBe(first)
    expect(result.configs.client[1]).toBe(second)
    expect(result.configs.nodejs[0]).toBe(first)
    expect(result.configs.edge[0]).toBe(first)
  },
)

it('leaves unseen owners absent when reading empty open metadata', () => {
  // Given
  const owner = createWebpackGeneration()
  // When
  const result = read(owner)
  // Then
  expect(result).toEqual({
    phase: 'open',
    configs: { client: [], nodejs: [], edge: [] },
  })
  expect(
    globalThis.__devupUiWebpackProductionManifests?.has(owner) ?? false,
  ).toBe(false)
})

it.each([false, true])(
  'seals only current membership when partial=%s',
  (partial) => {
    // Given
    const owner = createWebpackGeneration()
    const empty = read(owner)
    const config: Configuration = {}
    if (partial) register(owner, 'nodejs', config)
    const open = read(owner)
    // When
    const result = seal(owner)
    // Then
    expect(result.phase).toBe('sealed')
    expect(result.configs).toEqual({
      client: [],
      nodejs: partial ? [config] : [],
      edge: [],
    })
    expect(read(owner)).toBe(result)
    expect(seal(owner)).toBe(result)
    expect(empty.configs.nodejs).toEqual([])
    expect(open.phase).toBe('open')
    expect(owner.disposed).toBe(false)
    for (const snapshot of [empty, open, result]) {
      expect(Object.isFrozen(snapshot)).toBe(true)
      expect(Object.isFrozen(snapshot.configs)).toBe(true)
      for (const configs of Object.values(snapshot.configs))
        expect(Object.isFrozen(configs)).toBe(true)
    }
  },
)

it('retains native mutability when entries change before and after seal', () => {
  // Given
  const owner = createWebpackGeneration()
  const entry = { main: './first.ts' }
  const config: Configuration = { entry }
  register(owner, 'client', config)
  const before = read(owner)
  entry.main = './second.ts'
  // When
  const result = seal(owner)
  entry.main = './third.ts'
  config.entry = { other: './fourth.ts' }
  owner.dispose()
  // Then
  expect(before.configs.client[0]).toBe(config)
  expect(result.configs.client[0]).toBe(config)
  expect(result.configs.client[0]?.entry).toEqual({ other: './fourth.ts' })
  expect(entry.main).toBe('./third.ts')
  expect(Object.isFrozen(config)).toBe(false)
  expect(Object.isFrozen(entry)).toBe(false)
  expect(Object.isFrozen(result)).toBe(true)
})

it('never evaluates entries when registering, reading, sealing or rejecting', () => {
  // Given
  const owner = createWebpackGeneration()
  const config: Configuration = {
    get entry(): Configuration['entry'] {
      throw new Error('Entry evaluated')
    },
  }
  // When
  register(owner, 'edge', config)
  const open = read(owner)
  const sealed = seal(owner)
  const late = rejection(() => register(owner, 'edge', config))
  owner.dispose()
  const disposed = rejection(() => read(owner))
  // Then
  expect(open.configs.edge[0]).toBe(config)
  expect(sealed.configs.edge[0]).toBe(config)
  expect(late.code).toBe('sealed')
  expect(disposed.coordinates).toEqual([{ role: 'edge' }])
})

it.each([false, true])(
  'rejects every registration when sealed and identical=%s',
  (identical) => {
    // Given
    const owner = createWebpackGeneration()
    const config: Configuration = { context: '/actual', name: 'actual-client' }
    register(owner, 'client', config)
    const snapshot = seal(owner)
    const attempted = identical ? config : {}
    // When
    const error = rejection(() => register(owner, 'client', attempted))
    // Then
    expect(error.code).toBe('sealed')
    expect(error.operation).toBe('register')
    expect(error.coordinates).toEqual([
      identical
        ? { role: 'client', context: '/actual', name: 'actual-client' }
        : { role: 'client' },
    ])
    expect(Object.hasOwn(error, 'cause')).toBe(false)
    expect(Object.isFrozen(error.coordinates)).toBe(true)
    expect(Object.isFrozen(error.coordinates[0])).toBe(true)
    expect(read(owner)).toBe(snapshot)
  },
)

it.each(['unseen', 'open', 'sealed'] as const)(
  'rejects all access with disposed precedence when previously %s',
  (state) => {
    // Given
    const owner = createWebpackGeneration()
    const config: Configuration = { context: '/stored', name: 'stored' }
    if (state !== 'unseen') {
      register(owner, 'nodejs', config)
      register(owner, 'edge', {})
    }
    if (state === 'sealed') seal(owner)
    owner.dispose()
    const attempted: Configuration = { name: 'attempted' }
    // When
    const errors = [
      rejection(() => register(owner, 'client', attempted)),
      rejection(() => read(owner)),
      rejection(() => seal(owner)),
    ]
    // Then
    expect(errors.map((error) => [error.code, error.operation])).toEqual([
      ['disposed', 'register'],
      ['disposed', 'read'],
      ['disposed', 'seal'],
    ])
    expect(errors[0]?.coordinates).toEqual([
      { role: 'client', name: 'attempted' },
    ])
    for (const error of errors.slice(1)) {
      expect(error.coordinates).toEqual(
        state === 'unseen'
          ? []
          : [
              { role: 'nodejs', context: '/stored', name: 'stored' },
              { role: 'edge' },
            ],
      )
      expect(Object.isFrozen(error.coordinates)).toBe(true)
      expect(error.coordinates.every(Object.isFrozen)).toBe(true)
      expect(Object.hasOwn(error, 'cause')).toBe(false)
    }
  },
)

it('does not pin completion or alias fresh owners when unused roles are registered', () => {
  // Given
  const owner = createWebpackGeneration()
  const config: Configuration = { context: '/same', name: 'same' }
  register(owner, 'edge', config)
  register(owner, 'nodejs', config)
  const independent = createWebpackGeneration()
  const scope = bindCompilerScope({}, { owner, complete: true })
  scope.start()
  // When
  scope.close()
  const fresh = createWebpackGeneration()
  register(fresh, 'client', config)
  // Then
  expect(owner.disposed).toBe(true)
  expect(read(independent).configs).toEqual({
    client: [],
    nodejs: [],
    edge: [],
  })
  expect(read(fresh).configs).toEqual({
    client: [config],
    nodejs: [],
    edge: [],
  })
  expect(rejection(() => read(owner)).code).toBe('disposed')
})

it('preserves actual engine bytes and extraction when metadata runs in a bound scope', () => {
  // Given
  const owner = createWebpackGeneration()
  const scope = bindCompilerScope({}, { owner, complete: true })
  const capture = () => [
    wasm.exportSheet(),
    wasm.exportClassMap(),
    wasm.exportFileMap(),
    wasm.exportCanonicalMap(),
  ]
  const extract = () => {
    const output = wasm.codeExtract(
      'manifest.tsx',
      "import {Box} from '@devup-ui/react';export const x=<Box bg='red'/>",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    try {
      return [output.code, wasm.getCss(null, false)]
    } finally {
      output.free()
    }
  }
  const expected = scope.run(extract)
  // When
  const actual = scope.run(() => {
    const before = capture()
    register(owner, 'client', {})
    read(owner)
    seal(owner)
    expect(capture()).toEqual(before)
    return extract()
  })
  // Then
  expect(actual).toEqual(expected)
  expect(actual[1]).toContain('background:red')
  scope.abort()
})
