import { expect, it } from 'bun:test'

import { runAvailabilityIntegration } from '../../../next-plugin/src/__tests__/availability-harness'

it('W66 availability: keeps distinct shared buckets byte-stable through Vite hooks in AB/BA/BA/AB', () => {
  // Given: two physical shared suppliers, both reached by two private entries.
  // When: actual built adapter callbacks run in fresh processes at one fixed root.
  const [ab, ba, baAgain, abAgain] = runAvailabilityIntegration('vite')
  // Then: complete compiled JS/maps and loaded/emitted CSS are byte-identical.
  expect(ba).toEqual(ab)
  expect(baAgain).toEqual(ab)
  expect(abAgain).toEqual(ab)
}, 200_000)
