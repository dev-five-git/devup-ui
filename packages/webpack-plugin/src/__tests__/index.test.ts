describe('export', () => {
  it('should export DevupUIWebpackPlugin', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      DevupUIWebpackPlugin: expect.any(Function),
    })
  })
})
