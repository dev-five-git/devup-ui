import { expect, it } from 'bun:test'

import {
  reusableMdxBindingValue,
  snapshotMdxBindingValue,
} from '../mdx-binding-value'

const date = new Date('2026-01-01T00:00:00.000Z')

it.each([
  [{ nested: { date } }, true],
  [[date], false],
  [date, false],
  [{ nested: [date] }, true],
  [Object.defineProperty({}, 'date', { value: date }), false],
  [{ [Symbol('date')]: date }, false],
  [{ nested: Object.assign([], { date }) }, false],
  [{ nested: Object.assign([], { '01': date }) }, false],
  [{ nested: Object.assign([], { '4294967295': date }) }, false],
  [{ nested: Object.defineProperty([], '0', { value: date }) }, false],
] as const)(
  'requires a serialized live compiler path for Date input %#',
  (options, expected) => {
    // Given
    const snapshot = snapshotMdxBindingValue({ loader: 'compiler', options })
    // When
    const covered = reusableMdxBindingValue(snapshot, true)
    const ordinary = reusableMdxBindingValue(snapshot)
    // Then
    expect(covered).toBe(expected)
    expect(ordinary).toBe(false)
  },
)

it('rechecks a shared shell reached first through covered then hidden edges', () => {
  // Given
  const shared = { date }
  const options = Object.defineProperty({ covered: shared }, 'hidden', {
    value: shared,
  })
  // When
  const reusable = reusableMdxBindingValue(
    snapshotMdxBindingValue({ options }),
    true,
  )
  // Then
  expect(reusable).toBe(false)
})

it('terminates covered structural cycles without granting opaque raw coverage', () => {
  // Given
  const options: Record<string, unknown> = { value: 1 }
  options.self = options
  // When
  const reusable = reusableMdxBindingValue(
    snapshotMdxBindingValue({ options }),
    true,
  )
  // Then
  expect(reusable).toBe(true)
})
