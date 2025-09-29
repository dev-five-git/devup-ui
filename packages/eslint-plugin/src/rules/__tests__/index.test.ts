import * as index from '../index'
describe('export index', () => {
  it('export', () => {
    expect({ ...index }).toEqual({
      noUselessTailingNulls: expect.any(Object),
    })
  })
})
