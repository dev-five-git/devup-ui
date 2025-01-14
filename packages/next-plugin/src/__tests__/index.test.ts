describe('export', () => {
  it('should export DevupUI', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      DevupUI: expect.any(Function),
    })
  })
})
