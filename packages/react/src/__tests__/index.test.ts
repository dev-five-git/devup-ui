import { describe, expect, it } from 'bun:test'

describe('export', () => {
  it('should export components and css', async () => {
    const { styled, ...index } = await import('../index')
    // `styled` is a Proxy over Function.prototype whose `get` trap answers every
    // property, `constructor` included, so asymmetric matchers cannot describe it.
    expect(typeof styled).toBe('function')
    expect({ ...index }).toEqual({
      Box: expect.any(Function),
      Button: expect.any(Function),
      Center: expect.any(Function),
      Flex: expect.any(Function),
      Input: expect.any(Function),
      Text: expect.any(Function),
      VStack: expect.any(Function),
      Image: expect.any(Function),
      Grid: expect.any(Function),

      css: expect.any(Function),
      globalCss: expect.any(Function),
      keyframes: expect.any(Function),
      stylex: expect.any(Object),

      ThemeScript: expect.any(Function),

      getTheme: expect.any(Function),
      setTheme: expect.any(Function),
      initTheme: expect.any(Function),

      useTheme: expect.any(Function),
    })
  })
})
