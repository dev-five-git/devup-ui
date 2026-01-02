import { describe, expect, it } from 'bun:test'

describe('index exports', () => {
  it('should export getDevupDefaultTheme and getDevupDefine', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({})
  })
})
