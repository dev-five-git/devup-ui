import { describe, expect, it } from 'bun:test'

import { __devupNativeScale, __devupScale } from '../scale'

describe('__devupScale', () => {
  it.each([
    [2, '8px'],
    ['2', '8px'],
    [-1.5, '-6px'],
    ['+.5', '2px'],
    ['1e1', '40px'],
    [0, '0px'],
  ] as const)('converts %p as a literal of a length', (value, expected) => {
    expect(__devupScale(value)).toBe(expected)
  })

  it('takes the scale and unit of a duration', () => {
    expect(__devupScale(3, 1, 'ms')).toBe('3ms')
    expect(__devupScale('3', 1, 'ms')).toBe('3ms')
  })

  it.each([
    '2px',
    '2rem',
    'auto',
    '$space',
    '',
    ' 2',
    null,
    undefined,
    false,
  ] as const)('keeps %p as it is', (value) => {
    expect(__devupScale(value)).toBe(value)
  })

  it('runs no code of an object it is given', () => {
    const value = {
      toString(): string {
        throw new Error('called')
      },
    }
    expect(__devupScale(value)).toBe(value)
  })
})

describe('__devupNativeScale', () => {
  it.each([
    [2, 4, 8],
    ['2', 4, 8],
    [' -1.5 ', 4, -6],
    ['3px', 4, 3],
    ['2rem', 4, 32],
    ['1em', 1, 16],
    [2, 1, 2],
  ] as const)('converts %p by %p as a literal', (value, scale, expected) => {
    expect(__devupNativeScale(value, scale)).toBe(expected)
  })

  it.each(['auto', '2%', '', null, undefined, false] as const)(
    'keeps %p as it is',
    (value) => {
      expect(__devupNativeScale(value, 4)).toBe(value)
    },
  )
})
