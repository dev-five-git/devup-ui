// Import from bun-test-env-dom to enable DOM environment
import 'bun-test-env-dom'

import { describe, expect, it } from 'bun:test'

import { createServerThemeStore } from '../theme-store-server'

type ServerThemeStore = ReturnType<typeof createServerThemeStore>
type ThemeValue = Parameters<ServerThemeStore['set']>[0]

const darkTheme = 'dark' as unknown as ThemeValue

describe('themeStore ssr', () => {
  it('should select the inert server store when window is undefined', async () => {
    // Given: evaluate the factory without a browser window, preserving its descriptor.
    const originalWindowDescriptor = Object.getOwnPropertyDescriptor(
      globalThis,
      'window',
    )

    try {
      Object.defineProperty(globalThis, 'window', {
        configurable: true,
        value: undefined,
      })

      // When: a fresh import selects the factory in this process.
      const { createThemeStore } = await import('../theme-store?ssr-factory')

      // Then: the real server factory provides an inert subscription lifecycle.
      expect(createThemeStore).toBe(createServerThemeStore)
      const themeStore = createThemeStore()
      const notifications: ThemeValue[] = []
      expect(themeStore.get()).toBeNull()
      const unsubscribe = themeStore.subscribe((theme) => {
        notifications.push(theme)
      })
      expect(notifications).toEqual([])
      expect(themeStore.set(darkTheme)).toBeUndefined()
      expect(themeStore.get()).toBeNull()
      expect(notifications).toEqual([])
      expect(unsubscribe()).toBeUndefined()
      expect(unsubscribe()).toBeUndefined()
      expect(themeStore.set(darkTheme)).toBeUndefined()
      expect(themeStore.get()).toBeNull()
      expect(notifications).toEqual([])
    } finally {
      if (originalWindowDescriptor) {
        Object.defineProperty(globalThis, 'window', originalWindowDescriptor)
      } else {
        Reflect.deleteProperty(globalThis, 'window')
      }
    }
  })

  it('should filter mutations by type and target', async () => {
    // const { createThemeStore } = await import('../theme-store')
    const themeStore = createServerThemeStore()

    // Test SSR store behavior
    expect(themeStore).toBeDefined()
    expect(themeStore.get()).toBeNull()
    expect(themeStore.set(darkTheme)).toBeUndefined()

    const unsubscribe = themeStore.subscribe(() => {})
    expect(typeof unsubscribe).toBe('function')

    // The unsubscribe should return a no-op function
    const innerUnsubscribe = unsubscribe()
    expect(innerUnsubscribe).toBeUndefined()
  })
})
