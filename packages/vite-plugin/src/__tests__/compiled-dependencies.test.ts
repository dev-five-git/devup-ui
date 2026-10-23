import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { createCompiledDependencies } from '../compiled-dependencies'

it.each([
  ['unregistered', 'shared.ts'],
  ['client', 'unregistered.ts'],
])(
  'returns no consumers for %s / %s when unregistered',
  (environment, file) => {
    // Given
    const tracker = createCompiledDependencies()
    tracker.update('client', 'entry.tsx', ['shared.ts'])
    // When
    const consumers = tracker.consumers(environment, file)
    // Then
    expect(consumers).toEqual([])
  },
)

it('replaces all previous edges when a consumer is recompiled', () => {
  // Given
  const tracker = createCompiledDependencies()
  tracker.update('client', 'entry.tsx', ['old-a.ts', 'old-b.ts'])
  tracker.update('client', 'other.tsx', ['old-a.ts', 'old-b.ts'])
  // When
  tracker.update('client', 'entry.tsx', ['new.ts'])
  // Then
  expect(tracker.consumers('client', 'old-a.ts')).toEqual(['other.tsx'])
  expect(tracker.consumers('client', 'old-b.ts')).toEqual(['other.tsx'])
  expect(tracker.consumers('client', 'new.ts')).toEqual(['entry.tsx'])
})

it('removes all previous edges when recompilation has no suppliers', () => {
  // Given
  const tracker = createCompiledDependencies()
  tracker.update('client', 'entry.tsx', ['shared.ts'])
  tracker.update('client', 'other.tsx', ['shared.ts'])
  // When
  tracker.update('client', 'entry.tsx', [])
  // Then
  expect(tracker.consumers('client', 'shared.ts')).toEqual(['other.tsx'])
})

it('resolves supplier paths when registering and looking up consumers', () => {
  // Given
  const tracker = createCompiledDependencies()
  tracker.update('client', 'entry.tsx', ['src/nested/../shared.ts'])
  // When
  const consumers = tracker.consumers('client', resolve('src/shared.ts'))
  // Then
  expect(consumers).toEqual(['entry.tsx'])
})

it('resolves relative supplier lookups against registered absolute paths', () => {
  // Given
  const tracker = createCompiledDependencies()
  tracker.update('client', 'entry.tsx', [resolve('src/shared.ts')])
  // When
  const consumers = tracker.consumers('client', 'src/nested/../shared.ts')
  // Then
  expect(consumers).toEqual(['entry.tsx'])
})

it('isolates recompilation when environments share a physical consumer and supplier', () => {
  // Given
  const tracker = createCompiledDependencies()
  const consumer = resolve('entry.tsx')
  const supplier = resolve('shared.ts')
  tracker.update('client', consumer, [supplier])
  tracker.update('ssr', consumer, [supplier])
  // When
  tracker.update('client', consumer, ['client-only.ts'])
  // Then
  expect(tracker.consumers('client', supplier)).toEqual([])
  expect(tracker.consumers('ssr', supplier)).toEqual([consumer])
  expect(tracker.consumers('client', 'client-only.ts')).toEqual([consumer])
  expect(tracker.consumers('ssr', 'client-only.ts')).toEqual([])
})

it('unlinks query-bearing consumers across environments while preserving unrelated edges', () => {
  // Given
  const tracker = createCompiledDependencies()
  const consumer = resolve('src/entry.tsx')
  const other = resolve('src/entry.tsx.extra')
  tracker.update('client', `${consumer}?direct`, ['shared.ts', 'removed.ts'])
  tracker.update('client', `${consumer}?raw`, ['shared.ts'])
  tracker.update('ssr', `${consumer}?ssr`, ['shared.ts', 'removed.ts'])
  tracker.update('ssr', consumer, ['shared.ts'])
  tracker.update('client', other, ['shared.ts', 'other.ts', consumer])
  tracker.update('ssr', other, ['shared.ts', 'other.ts', consumer])
  // When
  tracker.remove('src/nested/../entry.tsx')
  // Then
  for (const environment of ['client', 'ssr']) {
    expect(tracker.consumers(environment, 'shared.ts')).toEqual([other])
    expect(tracker.consumers(environment, 'removed.ts')).toEqual([])
    expect(tracker.consumers(environment, 'other.ts')).toEqual([other])
    expect(tracker.consumers(environment, consumer)).toEqual([other])
  }
})
