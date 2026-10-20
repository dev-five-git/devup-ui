import assert from 'node:assert/strict'

import { expect, it } from 'bun:test'

import { fixture, record, run, tokenFixture } from './audit-fixture'

it('refreshes the shared manager on resolution-only changes before real engine extraction', () => {
  // Given
  using fs = fixture()
  tokenFixture(fs)
  // When: helper changes only default manifest target, with both leaves already selected.
  const result = run(fs.root, 'manager')
  // Then
  expect(result.sameInputs).toBe(true)
  expect(result.sameSources).toBe(true)
  record(result.before)
  record(result.after)
  expect(result.before.count).toBe(0)
  expect(result.after.count).toBe(0)
  expect(result.second).toBe(true)
}, 60000)

it('refreshes resolution-only revisions at repeated cache-disabled native run boundaries', () => {
  // Given
  using fs = fixture()
  tokenFixture(fs)
  // When
  const result = run(fs.root, 'revision')
  // Then
  expect(result.ok).toBe(true)
  expect(result.count).toBe(0)
  assert(Array.isArray(result.revisions))
  expect(result.revisions).toHaveLength(2)
}, 120000)
