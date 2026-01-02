import { describe, expect, it } from 'bun:test'

describe('export', () => {
  it('should export DevupUIVitePlugin', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      DevupUI: expect.any(Function),
    })
  })
})
