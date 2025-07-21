import { css } from '../css'

describe('css', () => {
  it('should return className', () => {
    expect(() => css`virtual-css`).toThrowError('Cannot run on the runtime')
    expect(() => css('class name' as any)).toThrowError(
      'Cannot run on the runtime',
    )
    expect(() => css()).toThrowError('Cannot run on the runtime')
  })
})
