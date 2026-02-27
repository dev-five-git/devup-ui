import { describe, expect, it } from 'bun:test'

import { create, firstThatWorks, keyframes, props, types } from '../stylex'

describe('stylex', () => {
  it('create should throw at runtime', () => {
    expect(() => create({ base: { color: 'red' } })).toThrowError(
      'Cannot run on the runtime',
    )
  })

  it('props should throw at runtime', () => {
    expect(() => props()).toThrowError('Cannot run on the runtime')
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
})
