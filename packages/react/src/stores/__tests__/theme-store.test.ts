beforeEach(() => {
  vi.resetModules()
})

describe('themeStore', () => {
  it('should return themeStore object for browser', async () => {
    const { createThemeStore } = await import('../theme-store')
    const themeStore = createThemeStore()
    expect(themeStore).toBeDefined()
    expect(themeStore.get).toEqual(expect.any(Function))
    expect(themeStore.set).toEqual(expect.any(Function))
    expect(themeStore.subscribe).toEqual(expect.any(Function))
    expect(themeStore.get()).toBeNull()
    expect(themeStore.set('dark' as any)).toBeUndefined()
    expect(themeStore.subscribe(() => {})()).toBeUndefined()
    themeStore.subscribe(() => {})
    expect(themeStore.set('dark' as any)).toBeUndefined()
  })
})
