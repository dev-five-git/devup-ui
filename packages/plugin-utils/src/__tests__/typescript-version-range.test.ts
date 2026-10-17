import { createRequire } from 'node:module'

import { expect, it } from 'bun:test'

import { matchesTypeScriptVersion } from '../typescript-version-range'

const ts: {
  getPackageJsonTypesVersionsPaths(
    value: object,
  ): { readonly version: string } | undefined
} = createRequire(import.meta.url)('@typescript/typescript6')
it.each([
  ['', true],
  ['*', true],
  ['||', true],
  ['x', true],
  ['X', true],
  ['6', true],
  ['6.0', true],
  ['6.0.3', true],
  ['=6.0.3', true],
  ['7', false],
  ['~6', true],
  ['~6.0', true],
  ['~6.0.3', true],
  ['~6.1', false],
  ['^6', true],
  ['^6.0.3', true],
  ['^0', false],
  ['^0.1', false],
  ['^0.0.3', false],
  ['<6', false],
  ['<=6', true],
  ['>6', false],
  ['>=6', true],
  ['<=6.0', true],
  ['>6.0', false],
  ['<6.0.3', false],
  ['<=6.0.3', true],
  ['>6.0.3', false],
  ['>=6.0.3', true],
  ['>5.9', true],
  ['<7.0', true],
  ['<*', false],
  ['>*', false],
  ['>=*', true],
  ['6 - 6', true],
  ['6.0 - 6.0', true],
  ['6.0.3 - 6.0.3', true],
  ['* - 6', true],
  ['6 - *', true],
  ['* - *', true],
  ['7 - 8', false],
  ['bad - 6', false],
  ['6 - bad', false],
  ['>=6.0.3-beta', true],
  ['>6.0.3-0', true],
  ['6.0.3+build', true],
  ['6.0.3-BETA.1', false],
  ['6.0.3+abc.1', true],
  ['>=5 <7', true],
  ['<5 || >=6', true],
  ['>=7 || <5', false],
  [' 6 ', true],
  ['>= 6', false],
  ['v6', false],
  ['6.0.3.4', false],
  ['01', false],
] as const)('tests pinned compiler version against %s', (range, expected) => {
  // Given a range and an independently specified answer for stable 6.0.3.
  const selection = ts.getPackageJsonTypesVersionsPaths({ [range]: {} })
  // When using the installed parser and the owned range evaluator.
  expect(selection !== undefined).toBe(expected)
  // Then both match the independent answer, including invalid ranges.
  expect(matchesTypeScriptVersion(range)).toBe(expected)
})

it.each(['6.0.3-01', '6.0.3-a..b', '6.0.3+..'])(
  'preserves compiler failure for malformed components in %s',
  (range) => {
    expect(() => ts.getPackageJsonTypesVersionsPaths({ [range]: {} })).toThrow()
    expect(() => matchesTypeScriptVersion(range)).toThrow(
      'Invalid TypeScript version range',
    )
  },
)
