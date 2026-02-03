// Import from bun-test-env-dom to enable DOM environment
import 'bun-test-env-dom'

import {
  afterAll,
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  mock,
} from 'bun:test'

import { createThemeStore } from '../theme-store'

beforeAll(() => {
  document.documentElement.removeAttribute('data-theme')
})

afterAll(() => {
  document.documentElement.removeAttribute('data-theme')
})

describe.each([
  [false, 'server'],
  [true, 'browser'],
])('themeStore %s', (_isBrowser, _title) => {
  const originalWindow = globalThis.window
  beforeEach(() => {
    // Remove window to simulate SSR
    if (!_isBrowser) {
      // @ts-expect-error - Temporarily remove window for SSR test
      delete globalThis.window
    } else {
      document.documentElement.removeAttribute('data-theme')
    }
  })

  afterEach(() => {
    if (_isBrowser) {
      document.documentElement.removeAttribute('data-theme')
    }
    globalThis.window = originalWindow
  })
  if (_isBrowser) {
    it('should return themeStore object for browser', () => {
      // const modulePath = require.resolve('../theme-store')
      // delete require.cache[modulePath]
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

    it('should call subscriber when theme changes via set', () => {
      // const modulePath = require.resolve('../theme-store')
      // delete require.cache[modulePath]
      const themeStore = createThemeStore()
      const callback = mock()

      themeStore.subscribe(callback)

      // First call is from subscribe itself (reads current data-theme)
      expect(callback).toHaveBeenCalledTimes(1)

      themeStore.set('light' as any)
      expect(callback).toHaveBeenCalledTimes(2)
      expect(callback).toHaveBeenLastCalledWith('light')
      expect(themeStore.get()).toBe('light' as any)
    })

    it('should unsubscribe correctly', () => {
      // const modulePath = require.resolve('../theme-store')
      // delete require.cache[modulePath]
      const themeStore = createThemeStore()
      const callback = mock()

      const unsubscribe = themeStore.subscribe(callback)
      expect(callback).toHaveBeenCalledTimes(1)

      // Unsubscribe
      const result = unsubscribe()
      expect(result).toBe(true as any)

      // Should not be called after unsubscribe
      themeStore.set('dark' as any)
      expect(callback).toHaveBeenCalledTimes(1)
    })

    it('should read initial theme from data-theme attribute', () => {
      document.documentElement.setAttribute('data-theme', 'dark')

      // const modulePath = require.resolve('../theme-store')
      // delete require.cache[modulePath]
      const themeStore = createThemeStore()
      const callback = mock()

      themeStore.subscribe(callback)

      // Should be called with 'dark' from the attribute
      expect(callback).toHaveBeenCalledWith('dark')
    })

    it('should update theme when data-theme attribute changes via MutationObserver', async () => {
      // const modulePath = require.resolve('../theme-store')
      // delete require.cache[modulePath]
      const themeStore = createThemeStore()
      const callback = mock()

      themeStore.subscribe(callback)
      expect(callback).toHaveBeenCalledTimes(1)

      // Change the attribute - MutationObserver should trigger
      document.documentElement.setAttribute('data-theme', 'dark')

      // Wait for MutationObserver to fire (it's async)
      await new Promise((resolve) => setTimeout(resolve, 10))

      expect(callback).toHaveBeenCalledWith('dark')
      expect(themeStore.get()).toBe('dark' as any)
    })

    it('should handle multiple subscribers', async () => {
      // const modulePath = require.resolve('../theme-store')
      // delete require.cache[modulePath]
      const themeStore = createThemeStore()
      const callback1 = mock()
      const callback2 = mock()

      themeStore.subscribe(callback1)
      themeStore.subscribe(callback2)

      themeStore.set('light' as any)

      expect(callback1).toHaveBeenLastCalledWith('light')
      expect(callback2).toHaveBeenLastCalledWith('light')
    })

    it('should filter mutations by type and target', async () => {
      // const modulePath = require.resolve('../theme-store')
      // delete require.cache[modulePath]
      const themeStore = createThemeStore()
      const callback = mock()

      themeStore.subscribe(callback)

      // Change data-theme attribute (should trigger)
      document.documentElement.setAttribute('data-theme', 'system')

      await new Promise((resolve) => setTimeout(resolve, 10))

      expect(themeStore.get()).toBe('system' as any)
    })
  } else {
    it('should return themeStore object for server', async () => {
      const { createThemeStore: createThemeStoreFn } = require.cache[
        require.resolve('../theme-store')
      ]?.exports as unknown as { createThemeStore: typeof createThemeStore }
      const themeStore = createThemeStoreFn()

      // Test SSR store behavior
      expect(themeStore).toBeDefined()
      expect(themeStore.get()).toBeNull()
      expect(themeStore.set('dark' as any)).toBeUndefined()

      const unsubscribe = themeStore.subscribe(() => {})
      expect(typeof unsubscribe).toBe('function')

      // The unsubscribe should return a no-op function
      const innerUnsubscribe = unsubscribe()
      expect(innerUnsubscribe).toBeUndefined()
    })
  }
})
