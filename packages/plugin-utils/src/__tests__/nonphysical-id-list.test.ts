import { expect, it, spyOn } from 'bun:test'

import {
  nextNonphysicalIdList,
  parseNonphysicalIdList,
  serializeNonphysicalIdList,
} from '../nonphysical-id-list'

it.each([
  { serialized: undefined, expected: { kind: 'missing', ids: [] } },
  { serialized: '[]', expected: { kind: 'loaded', ids: [] } },
  {
    serialized: '["z","a","z","__proto__","constructor"]',
    expected: { kind: 'loaded', ids: ['__proto__', 'a', 'constructor', 'z'] },
  },
])(
  'returns the typed outcome for $serialized with immutable set contents',
  ({ serialized, expected }) => {
    // Given absent or valid strict serialized ID arrays.
    // When parsing at the serialized boundary.
    const result = parseNonphysicalIdList(serialized)
    // Then outcome distinguishes absence from a valid empty list, with frozen data.
    expect(result).toEqual(expected)
    expect(Object.isFrozen(result)).toBe(true)
    expect(Object.isFrozen(result.ids)).toBe(true)
  },
)

it.each([
  '',
  ' ',
  '[',
  '["valid",]',
  '// comment\n[]',
  'null',
  '{}',
  '"id"',
  '0',
  'true',
  '{"version":1,"files":{"id":0}}',
  '["valid",0]',
  '["valid",null]',
  '["valid",{}]',
  '["valid",true]',
  '["valid",[]]',
  '["valid",""]',
  '["", "valid"]',
])(
  'rejects the entire corrupt list %j without partial salvage',
  (serialized) => {
    // Given malformed JSON, a non-array or an invalid array element.
    // When real JSON parsing and element narrowing run.
    const result = parseNonphysicalIdList(serialized)
    // Then corruption becomes a typed frozen empty outcome, not a build error.
    expect(result).toEqual({ kind: 'corrupt', ids: [] })
    expect(Object.isFrozen(result)).toBe(true)
    expect(Object.isFrozen(result.ids)).toBe(true)
  },
)

it('preserves opaque whitespace Unicode suffix NUL and backslash IDs while parsing', () => {
  // Given independently spelled opaque normalized extraction IDs.
  const serialized =
    '["😀","é","é","文件","virtual:x?raw#x","\\u0000id","back\\\\slash"," ","é"]'
  // When parsing real strict JSON once.
  const result = parseNonphysicalIdList(serialized)
  // Then exact values survive, with duplicates removed and UTF-16 ordering.
  expect(result).toEqual({
    kind: 'loaded',
    ids: [
      '\0id',
      ' ',
      'back\\slash',
      'e\u0301',
      'virtual:x?raw#x',
      'é',
      '文件',
      '😀',
    ],
  })
})

it('rethrows the same unrelated boundary fault instead of reporting corruption', () => {
  // Given the explicitly authorized unavailable JSON.parse runtime fault.
  const fault = new TypeError('unrelated parser boundary fault')
  const parse = spyOn(JSON, 'parse').mockImplementation(() => {
    throw fault
  })
  try {
    // When parsing an otherwise valid string encounters this non-SyntaxError.
    let caught: unknown
    try {
      parseNonphysicalIdList('[]')
    } catch (error) {
      caught = error
    }
    // Then the original fault escapes unchanged, not as an empty corrupt list.
    expect(caught).toBe(fault)
  } finally {
    parse.mockRestore()
  }
})

interface NextCase {
  readonly label: string
  readonly extracted: readonly string[]
  readonly known: readonly string[]
  readonly expected: readonly string[]
}
const nextCases: readonly NextCase[] = [
  {
    label: 'saved-but-still-scan-missed survives while saved-only stale drops',
    extracted: ['savedUnknown', 'newUnknown', 'savedUnknown'],
    known: ['known'],
    expected: ['newUnknown', 'savedUnknown'],
  },
  {
    label: 'promoted known ID drops even when currently extracted',
    extracted: ['promotedKnown', 'currentUnknown'],
    known: ['promotedKnown'],
    expected: ['currentUnknown'],
  },
  {
    label:
      'physical-promoted saved ID is absent from the caller nonphysical cohort',
    extracted: ['currentUnknown'],
    known: [],
    expected: ['currentUnknown'],
  },
  {
    label: 'an empty build drops every saved-only stale ID',
    extracted: [],
    known: [],
    expected: [],
  },
  {
    label: 'all currently extracted IDs are now known',
    extracted: ['known', 'known'],
    known: ['known', 'known'],
    expected: [],
  },
  {
    label: 'deduplicates opaque values without trimming or renormalizing',
    extracted: [' ', '\0x?raw', ' ', 'a\\b'],
    known: ['other'],
    expected: ['\0x?raw', ' ', 'a\\b'],
  },
]
it.each([...nextCases])(
  'replaces the next list when $label',
  ({ extracted, known, expected }) => {
    // Given only actual extracted nonphysical and complete known-scan cohorts.
    // When replacing the store set without any previous-list argument.
    const result = nextNonphysicalIdList(extracted, known)
    // Then only unique current scan-missed IDs remain, frozen and detached.
    expect(result).toEqual(expected)
    expect(Object.isFrozen(result)).toBe(true)
    expect(result).not.toBe(extracted)
  },
)

it('keeps saved scan-missed IDs independent of changed physical seed metadata', () => {
  // Given caller fixtures with different physical seeds but the same cohorts.
  const fixtures = [
    {
      physical: ['oldPhysical'],
      buckets: ['oldBucket'],
      extracted: ['savedUnknown'],
      known: [],
    },
    {
      physical: ['newPhysical', 'extraPhysical'],
      buckets: ['newBucket'],
      extracted: ['savedUnknown'],
      known: [],
    },
  ]
  // When applying only the two authorized nonphysical cohorts.
  const results = fixtures.map(({ extracted, known }) =>
    nextNonphysicalIdList(extracted, known),
  )
  // Then metadata neither changes ID membership nor promises stable Rust numbers.
  expect(results).toEqual([['savedUnknown'], ['savedUnknown']])
})

it('does not mutate input cohorts and returns a frozen copy', () => {
  // Given mutable arrays in noncanonical arrival order.
  const extracted = ['z', 'known', 'a', 'z']
  const known = ['known']
  // When computing the exact current difference.
  const result = nextNonphysicalIdList(extracted, known)
  // Then inputs are unchanged and subsequent input edits cannot alter the result.
  expect(extracted).toEqual(['z', 'known', 'a', 'z'])
  expect(known).toEqual(['known'])
  extracted.push('late')
  known.push('a')
  expect(result).toEqual(['a', 'z'])
  expect(Object.isFrozen(result)).toBe(true)
})

it.each([
  {
    ids: [
      '\ue000',
      '😀',
      'é',
      'e\u0301',
      'z',
      '\0',
      ' ',
      'back\\slash',
      'virtual:x?raw#fragment',
      'é',
    ],
  },
  {
    ids: [
      'é',
      'virtual:x?raw#fragment',
      'back\\slash',
      ' ',
      '\0',
      'z',
      'e\u0301',
      'é',
      '😀',
      '\ue000',
    ],
  },
])(
  'serializes opposite arrival orders to identical canonical ID-only bytes',
  ({ ids }) => {
    // Given the same opaque ID set in independent arrival order with duplicates.
    // When serializing a copied set using UTF-16 lexical order, not Rust order.
    const serialized = serializeNonphysicalIdList(ids)
    // Then independently specified compact JSON contains exact IDs and no envelope.
    expect(serialized).toBe(
      '["\\u0000"," ","back\\\\slash","é","virtual:x?raw#fragment","z","é","😀",""]',
    )
  },
)

it('serializes the empty list as a bare JSON array', () => {
  // Given no current scan-missed extracted IDs.
  const ids: readonly string[] = []
  // When serializing that ID set.
  const serialized = serializeNonphysicalIdList(ids)
  // Then no domain, counter, version or newline is added.
  expect(serialized).toBe('[]')
})

it('copies serialization inputs without changing their order or duplicates', () => {
  // Given an unsorted mutable input array.
  const ids = ['z', 'a', 'z']
  // When serializing its unique copied set.
  const serialized = serializeNonphysicalIdList(ids)
  // Then caller order/multiplicity remain unchanged and bytes are canonical.
  expect(ids).toEqual(['z', 'a', 'z'])
  ids.push('late')
  expect(serialized).toBe('["a","z"]')
})

it('round-trips canonical opaque list bytes without Unicode normalization', () => {
  // Given a canonical independently specified list with distinct Unicode forms.
  const serialized = '["é","é","😀"]'
  // When composing parsing and serialization at the storage boundary.
  const rewritten = serializeNonphysicalIdList(
    parseNonphysicalIdList(serialized).ids,
  )
  // Then opaque identity spelling survives the additional composition case.
  expect(rewritten).toBe(serialized)
})
