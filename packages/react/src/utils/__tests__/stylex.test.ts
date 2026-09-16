import { describe, expect, it } from 'bun:test'

import {
  attrs,
  create,
  createTheme,
  createThemeContract,
  defineConsts,
  defineVars,
  firstThatWorks,
  include,
  keyframes,
  positionTry,
  props,
  types,
  viewTransitionClass,
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
    expect(() => viewTransitionClass({ opacity: '0' })).toThrowError(
      'Cannot run on the runtime',
    )
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
    expect(() => include({ color: 'red' })).toThrowError(
      'Cannot run on the runtime',
    )
  })

  it('defineVars should throw at runtime', () => {
    expect(() => defineVars({ primary: 'red' })).toThrowError(
      'Cannot run on the runtime',
    )
  })

  it('createTheme should throw at runtime', () => {
    expect(() =>
      createTheme({ primary: '--x' }, { primary: 'blue' }),
    ).toThrowError('Cannot run on the runtime')
  })
})
