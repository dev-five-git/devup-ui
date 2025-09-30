import * as index from '../index'
describe('export index', () => {
  it('export', () => {
    expect({ ...index }).toEqual({
      rules: expect.any(Object),
      configs: {
        recommended: expect.any(Object),
      },
      default: {
        configs: {
          recommended: expect.any(Object),
        },
      },
    })
  })
})
