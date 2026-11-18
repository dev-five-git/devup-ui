import { describe, expect, it } from 'bun:test'

import {
  attrs,
  create,
  createTheme,
  createThemeContract,
  defaultMarker,
  defineConsts,
  defineMarker,
  defineVars,
  firstThatWorks,
  include,
  keyframes,
  positionTry,
  props,
  types,
  viewTransitionClass,
  when,
} from '../stylex'

describe('stylex', () => {
  it('create should throw at runtime', () => {
    expect(() => create({ base: { color: 'red' } })).toThrowError(
      'Cannot run on the runtime',
    )
  })

  it('props should throw at runtime', () => {
    expect(() => props()).toThrowError('Cannot run on the runtime')
  })

  it('attrs should throw at runtime', () => {
    expect(() => attrs()).toThrowError('Cannot run on the runtime')
  })

  it('theme and at-rule helpers should throw at runtime', () => {
    expect(() => createThemeContract({ p: 'red' })).toThrowError(
      'Cannot run on the runtime',
    )
    expect(() => defineConsts({ p: 'red' })).toThrowError(
      'Cannot run on the runtime',
    )
    expect(() => positionTry({ top: '0' })).toThrowError(
      'Cannot run on the runtime',
    )
    expect(() => viewTransitionClass({ old: { opacity: '0' } })).toThrowError(
      'Cannot run on the runtime',
    )
  })

  it('markers and when conditions should throw at runtime', () => {
    expect(() => defineMarker()).toThrowError('Cannot run on the runtime')
    expect(() => defaultMarker()).toThrowError('Cannot run on the runtime')
    for (const relation of [
      when.ancestor,
      when.descendant,
      when.anySibling,
      when.siblingBefore,
      when.siblingAfter,
    ]) {
      expect(() => relation(':hover')).toThrowError('Cannot run on the runtime')
    }
  })

  it('keyframes should throw at runtime', () => {
    expect(() =>
      keyframes({ from: { opacity: '0' }, to: { opacity: '1' } }),
    ).toThrowError('Cannot run on the runtime')
  })

  it('firstThatWorks should throw at runtime', () => {
    expect(() => firstThatWorks('red', 'blue')).toThrowError(
      'Cannot run on the runtime',
    )
  })

  it('types.length should throw at runtime', () => {
    expect(() => types.length('10px')).toThrowError('Cannot run on the runtime')
  })

  it('types.color should throw at runtime', () => {
    expect(() => types.color('red')).toThrowError('Cannot run on the runtime')
  })

  it('include should throw at runtime', () => {
    expect(() =>
      Reflect.apply(include, undefined, [{ color: 'red' }]),
    ).toThrowError('Cannot run on the runtime')
  })

  it('defineVars should throw at runtime', () => {
    expect(() => defineVars({ primary: 'red' })).toThrowError(
      'Cannot run on the runtime',
    )
  })

  it('createTheme should throw at runtime', () => {
    expect(() =>
      Reflect.apply(createTheme, undefined, [
        { primary: '--x' },
        { primary: 'blue' },
      ]),
    ).toThrowError('Cannot run on the runtime')
  })
})
