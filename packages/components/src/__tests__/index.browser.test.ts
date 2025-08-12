describe('export', () => {
  it('should export components', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      Button: expect.any(Function),
      Input: expect.any(Function),
      Stepper: expect.any(Function),
    })
  })
})
