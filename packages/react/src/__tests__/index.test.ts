describe('export', () => {
  it('should export components and css', async () => {
    const index = await import('../index')
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

      ThemeScript: expect.any(Function),

      getTheme: expect.any(Function),
      setTheme: expect.any(Function),

      useTheme: expect.any(Function),
    })
  })
})
