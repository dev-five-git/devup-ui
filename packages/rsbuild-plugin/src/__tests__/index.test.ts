describe('export', () => {
  it('should export DevupUIVitePlugin', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      DevupUIRsbuildPlugin: expect.any(Function),
    })
  })
})
