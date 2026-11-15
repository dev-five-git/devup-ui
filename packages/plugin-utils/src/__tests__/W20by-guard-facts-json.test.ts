import { expect, it } from 'bun:test'

import type { GuardObservation } from '../guard-facts'
import { decodeGuardFacts } from '../guard-facts-json'

it('decodes reference origins, local exports and forwarding without an observer', () => {
  const reference = { request: 'ui', ids: ['css'], pos: 12 }
  const origin = { request: 'ui', ids: ['Box'] }
  const facts = decodeGuardFacts(
    JSON.stringify({
      references: [reference],
      exports: [
        ['Widget', origin],
        ['local', null],
      ],
      stars: ['forward'],
    }),
  )
  expect(facts).toEqual({
    references: [reference],
    exports: new Map([
      ['Widget', origin],
      ['local', undefined],
    ]),
    stars: ['forward'],
  })
})

it.each([false, true])(
  'retains observation sets with import-only metadata present=%s',
  (importOnly) => {
    const observations: GuardObservation[] = []
    const observation = {
      defaultCallable: true,
      unsupportedRequests: ['attributes', 'attributes'],
      ...(importOnly ? { importOnlyRequests: ['side-effect'] } : {}),
    }
    const facts = decodeGuardFacts(
      JSON.stringify({ references: [], exports: [], stars: [], observation }),
      (value) => observations.push(value),
    )
    expect(facts).toEqual({ references: [], exports: new Map(), stars: [] })
    expect(observations).toEqual([
      {
        defaultCallable: true,
        unsupportedRequests: new Set(['attributes']),
        ...(importOnly ? { importOnlyRequests: new Set(['side-effect']) } : {}),
      },
    ])
  },
)

it('does not manufacture an observation when the wire format omits it', () => {
  const observations: GuardObservation[] = []
  decodeGuardFacts(
    JSON.stringify({ references: [], exports: [], stars: [] }),
    (value) => observations.push(value),
  )
  expect(observations).toEqual([])
})
