import { expect, it } from 'bun:test'

import { runAvailabilityIntegration } from './availability-harness'

it('W66 availability: keeps distinct shared buckets byte-stable through Next live loaders in AB/BA/BA/AB', () => {
  // Given: two physical shared suppliers, both reached by two private pages.
  // When: actual built adapter callbacks run in fresh processes at one fixed root.
  const [ab, ba, baAgain, abAgain] = runAvailabilityIntegration('next')
  // Then: complete compiled JS/maps and actual loaded CSS are byte-identical.
  expect(ba).toEqual(ab)
  expect(baAgain).toEqual(ab)
  expect(abAgain).toEqual(ab)
}, 200_000)
