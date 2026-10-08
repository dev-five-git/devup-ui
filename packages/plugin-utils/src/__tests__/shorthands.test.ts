import { readFileSync } from 'node:fs'

import { describe, expect, it } from 'bun:test'

import {
  BUILTIN_SHORTHAND_TARGETS,
  CSS_PROPERTY_NAMES,
} from '../property-names'
import {
  type CustomShorthandTarget,
  isCustomShorthandTarget,
  normalizeShorthands,
  ShorthandConfigError,
} from '../shorthands'
import type { CustomShorthands } from '../types'

type Accepted<T> = T extends CustomShorthandTarget ? true : false
type ConfigAccepted<T> = T extends CustomShorthands ? true : false
const configProof: [
  ConfigAccepted<{ bgx: readonly ['backgroundColour'] }>,
  ConfigAccepted<{ bgx: readonly ['backgroundColor'] }>,
  ConfigAccepted<{ bgx: readonly ['background-color', 'py'] }>,
] = [false, true, true]
const typeProof: [
  Accepted<'marginRight'>,
  Accepted<'margin-right'>,
  Accepted<'py'>,
  Accepted<'-webkit-mask-image'>,
  Accepted<'--Gap'>,
  Accepted<'widht'>,
  Accepted<'backgroundColour'>,
  Accepted<'styleOrder'>,
  Accepted<'_hover'>,
  Accepted<'boxAlign'>,
  Accepted<'box-align'>,
] = [true, true, true, true, true, false, false, false, false, false, false]

describe('shorthand targets', () => {
  it('constrains configuration target types', () => {
    const config = {
      edges: ['left', 'py', 'margin-right', 'WebkitMaskImage', '--Gap'],
    } as const satisfies CustomShorthands
    expect(normalizeShorthands(config)).toEqual({
      edges: [
        'left',
        'padding-top',
        'padding-bottom',
        'margin-right',
        '-webkit-mask-image',
        '--Gap',
      ],
    })
    expect(typeProof).toEqual([
      true,
      true,
      true,
      true,
      true,
      false,
      false,
      false,
      false,
      false,
      false,
    ])
    expect(configProof).toEqual([false, true, true])
  })

  it('preserves readonly input, custom property case and empty lists', () => {
    const input = Object.freeze({
      z: Object.freeze(['--My-color', '--1', '--색']),
      empty: Object.freeze([]),
    })
    expect(normalizeShorthands(input)).toEqual({
      empty: [],
      z: ['--My-color', '--1', '--색'],
    })
    expect(Object.keys(normalizeShorthands(input))).toEqual(['empty', 'z'])
  })

  it.each([
    'marginRight',
    'margin-right',
    'WebkitMaskImage',
    '-webkit-mask-image',
    'MozAppearance',
    '-moz-appearance',
    'msUserSelect',
    '-ms-user-select',
    'py',
  ])('accepts the exact supported spelling %s', (target) => {
    expect(isCustomShorthandTarget(target)).toBe(true)
  })

  it.each([
    'widht',
    'scrollMargnLeft',
    'Width',
    'webkitMaskImage',
    'MsUserSelect',
    'OTransform',
    '_hover',
    'styleOrder',
    'positioning',
    'otherAlias',
    '',
    ' width',
    '--',
    '--bad name',
    1,
    null,
  ])('rejects an unsupported target %s', (target) => {
    expect(isCustomShorthandTarget(target)).toBe(false)
    expect(() => normalizeShorthands({ bad: ['width', target] })).toThrow(
      ShorthandConfigError,
    )
  })

  it('retains original target and logical configuration location', () => {
    const input = { aValid: ['right'], insetX: ['left', 'widht'] }
    try {
      normalizeShorthands(input)
      throw new Error('invalid target was accepted')
    } catch (error) {
      if (!(error instanceof ShorthandConfigError)) throw error
      expect(error.alias).toBe('insetX')
      expect(error.index).toBe(1)
      expect(error.target).toBe('widht')
      expect(error.path).toBe('shorthands["insetX"][1]')
      expect(error.name).toBe('ShorthandConfigError')
      expect(error.message).toBe(
        'devup-ui option shorthands["insetX"][1]: shorthand "insetX" cannot use "widht" at build time: its targets must be supported CSS properties, built-in aliases or custom properties',
      )
    }
    expect(input).toEqual({ aValid: ['right'], insetX: ['left', 'widht'] })
  })

  it.each([[null], [[]], ['bad'], [1]])(
    'rejects a malformed map %s',
    (input) => {
      expect(() => normalizeShorthands(input)).toThrow(ShorthandConfigError)
    },
  )

  it('rejects nonarray target lists', () => {
    expect(() => normalizeShorthands({ insetX: 'left' })).toThrow(
      ShorthandConfigError,
    )
  })

  it('keeps Rust canonical names and builtin expansions consistent', () => {
    const rust = readFileSync(
      new URL('../../../../libs/css/src/property_names.rs', import.meta.url),
      'utf8',
    )
    const canonical = [...rust.matchAll(/"([^"]+)"/g)]
      .map((match) => match[1])
      .sort()
    const normalized = normalizeShorthands({ names: CSS_PROPERTY_NAMES })
    expect([...normalized.names].sort()).toEqual(canonical)
    const aliases = readFileSync(
      new URL('../../../../libs/css/src/constant.rs', import.meta.url),
      'utf8',
    ).split('pub(super) static GLOBAL_ENUM_STYLE_PROPERTY')[0]
    const expansions = Object.fromEntries(
      [...aliases.matchAll(/"([^"]+)" => &\[([^\]]+)\]/g)].map((match) => [
        match[1],
        [...match[2].matchAll(/"([^"]+)"/g)].map((value) => value[1]),
      ]),
    )
    const builtinTargets: Record<string, readonly string[]> =
      BUILTIN_SHORTHAND_TARGETS
    expect(builtinTargets).toEqual(expansions)
  })

  it('rejects all eighteen unprefixed dead names', () => {
    for (const target of 'box-align,box-pack,box-flex,box-flex-group,box-orient,box-ordinal-group,box-direction,box-lines,flex-order,flex-positive,flex-negative,flex-preferred-size,scroll-snap-coordinate,scroll-snap-destination,scroll-snap-points-x,scroll-snap-points-y,scroll-snap-type-x,scroll-snap-type-y'.split(
      ',',
    )) {
      expect(isCustomShorthandTarget(target)).toBe(false)
      expect(
        isCustomShorthandTarget(
          target.replace(/-([a-z])/g, (_, letter: string) =>
            letter.toUpperCase(),
          ),
        ),
      ).toBe(false)
    }
  })
})
