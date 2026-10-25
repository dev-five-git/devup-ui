import { expect, it } from 'bun:test'

import {
  beginBuild,
  resetOwnedBuildState,
} from '../../../plugin-utils/src/build-admission.cjs'
import {
  nextNonphysicalIdList,
  parseNonphysicalIdList,
  serializeNonphysicalIdList,
} from '../../../plugin-utils/src/nonphysical-id-list'
import { decodeStringLiteral } from '../../../plugin-utils/src/string-literal'

it('resets an idle owner but retains a live legacy build', () => {
  // Given an idle engine and its real admission interval.
  let resets = 0
  const engine = {
    resetBuildState: () => {
      resets++
    },
  }
  // When cleanup runs before, during, and after the interval.
  resetOwnedBuildState(engine)
  const end = beginBuild(engine)
  try {
    resetOwnedBuildState(engine)
    expect(resets).toBe(2)
  } finally {
    end()
  }
  resetOwnedBuildState(engine)
  // Then only the idle cleanups and initial admission reset the engine.
  expect(resets).toBe(3)
})

it.each([
  [undefined, { kind: 'missing', ids: [] }],
  ['{', { kind: 'corrupt', ids: [] }],
  ['{}', { kind: 'corrupt', ids: [] }],
  ['[""]', { kind: 'corrupt', ids: [] }],
  ['[1]', { kind: 'corrupt', ids: [] }],
  ['["z","a","z"]', { kind: 'loaded', ids: ['a', 'z'] }],
] as const)(
  'classifies persisted nonphysical inventory %s',
  (serialized, expected) => {
    // Given persisted inventory bytes at their parsing boundary.
    // When V reads them.
    const outcome = parseNonphysicalIdList(serialized)
    // Then corruption cannot manufacture an owned ID.
    expect(outcome).toEqual(expected)
  },
)

it('serializes only deduplicated newly extracted nonphysical IDs', () => {
  // Given observed extraction IDs and already scanned IDs.
  const next = nextNonphysicalIdList(['z', 'a', 'z', 'known'], ['known'])
  // When persistence serializes the next inventory.
  const serialized = serializeNonphysicalIdList([...next, 'a'])
  // Then the stable bytes contain only the new sorted IDs.
  expect(serialized).toBe('["a","z"]')
})

it.each([
  ['"plain"', 'plain'],
  [String.raw`"\b\f\n\r\t\v"`, '\b\f\n\r\t\v'],
  [String.raw`"\0\101\77\400\8\9\z"`, '\0A? 089z'],
  ['"a\\\rb"', 'ab'],
  ['"a\\\r\nb"', 'ab'],
  ['"a\\\nb"', 'ab'],
  ['"a\\\u2028b\\\u2029c"', 'abc'],
  [String.raw`"\uD83D\uDE00"`, '😀'],
  [String.raw`"\u{1F600}"`, '😀'],
  [String.raw`"\xFf"`, 'ÿ'],
  [String.raw`"\u{110000}"`, undefined],
  [String.raw`"\xF"`, undefined],
  [String.raw`"\u00"`, undefined],
  [String.raw`"\u{41"`, undefined],
  ['"trailing\\"', undefined],
  ['"a\nb"', undefined],
  ['"a\rb"', undefined],
])('decodes or rejects boundary string literal %s', (literal, expected) => {
  // Given quoted import-specifier bytes.
  // When the decoder consumes them without evaluation.
  const value = decodeStringLiteral(literal)
  // Then escapes decode exactly and malformed tokens have no value.
  expect(value).toBe(expected)
})
