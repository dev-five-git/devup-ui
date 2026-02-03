import { describe, expect, it } from 'bun:test'

// This test file runs in isolation to test the SSR code path
// We need to mock window before importing the module

describe('themeStore SSR', () => {
  it('should return SSR-safe store when window is undefined', async () => {
    // Save original window
    const originalWindow = globalThis.window

    // Remove window to simulate SSR
    // @ts-expect-error - Temporarily remove window for SSR test
    delete globalThis.window

    // Clear module cache to force fresh import
    // const modulePath = require.resolve('../theme-store')
    // delete require.cache[modulePath]

    try {
      // Import with window undefined
      const { createThemeStore } = await import('../theme-store')
      const themeStore = createThemeStore()

      // Test SSR store behavior
      expect(themeStore).toBeDefined()
      expect(themeStore.get()).toBeNull()
      expect(themeStore.set('dark' as any)).toBeUndefined()

      const unsubscribe = themeStore.subscribe(() => {})
      expect(typeof unsubscribe).toBe('function')

      // The unsubscribe should return a no-op function
      const innerUnsubscribe = unsubscribe()
      expect(innerUnsubscribe).toBeUndefined()
    } finally {
      // Restore window
      globalThis.window = originalWindow
    }
  })
})
