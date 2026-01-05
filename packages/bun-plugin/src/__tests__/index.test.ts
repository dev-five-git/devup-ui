import { describe, expect, it } from 'bun:test'

describe('index exports', () => {
  it('should export nothing', async () => {
    const index = await import('../index')
    expect(index).toEqual({
      plugin: expect.any(Function),
    })
  })
})
