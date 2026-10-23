import { expect, it } from 'bun:test'

import { decodeStringLiteral } from '../string-literal'

// Given
it.each([
  ['"plain"', 'plain'],
  [String.raw`"\b\f\n\r\t\v"`, '\b\f\n\r\t\v'],
  [String.raw`"\0\101\77\400\8\9\z"`, '\0A? 089z'],
  ['"a\\\rb"', 'ab'],
  ['"a\\\r\nb"', 'ab'],
  ['"a\\\u2028b\\\u2029c"', 'abc'],
  [String.raw`"\uD83D\uDE00"`, '😀'],
  [String.raw`"\u{1F600}"`, '😀'],
  [String.raw`"\u{10FFFF}"`, String.fromCodePoint(0x10ffff)],
  [String.raw`"\xFf"`, 'ÿ'],
  [String.raw`"\xF"`, undefined],
  [String.raw`"\u00"`, undefined],
  [String.raw`"\u{41"`, undefined],
  ['"trailing\\"', undefined],
  ['"a\nb"', undefined],
  ['"a\rb"', undefined],
])(
  'decodes literal contents of %s without evaluating source',
  (literal, expected) => {
    // When
    const value = decodeStringLiteral(literal)
    // Then
    expect(value).toBe(expected)
  },
)
