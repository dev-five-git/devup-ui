import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

beforeEach(() => {
  document.documentElement.removeAttribute('data-theme')
})

afterEach(() => {
  document.documentElement.removeAttribute('data-theme')
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
    // subscribe returns an unsubscribe function, which returns boolean when called
    const unsubscribe = themeStore.subscribe(() => {})
    expect(typeof unsubscribe).toBe('function')
    themeStore.subscribe(() => {})
    expect(themeStore.set('dark' as any)).toBeUndefined()
  })
})
