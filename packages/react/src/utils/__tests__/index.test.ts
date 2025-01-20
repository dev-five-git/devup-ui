import { css } from '../css'

describe('css', () => {
  it('should return className', async () => {
    expect(css`virtual-css`).toEqual('virtual-css')
    expect(css('class name' as any)).toEqual('class name')
  })
})
