import { keyframes } from '../keyframes'

describe('keyframes', () => {
  it('should return animation', () => {
    expect(() =>
      keyframes({
        from: {
          opacity: 0,
        },
        to: {
          opacity: 1,
        },
      }),
    ).toThrowError('Cannot run on the runtime')
    expect(() => keyframes`from{opacity:0}to{opacity:1}`).toThrowError(
      'Cannot run on the runtime',
    )
  })
})
