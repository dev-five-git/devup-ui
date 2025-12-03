import { styled } from '../styled'

describe('globalCss', () => {
  it('should return className', () => {
    expect(() => styled.div`virtual-css`).toThrowError(
      'Cannot run on the runtime',
    )
  })
})
