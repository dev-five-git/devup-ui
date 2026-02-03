// Import from bun-test-env-dom to enable DOM environment
import 'bun-test-env-dom'

import { beforeAll } from 'bun:test'
import { afterAll } from 'bun:test'
import { describe, expect, it } from 'bun:test'

import { createServerThemeStore } from '../theme-store'

describe('themeStore ssr', () => {
  const originalWindow = globalThis.window
  beforeAll(() => {
    globalThis.window = undefined
  })

  afterAll(() => {
    globalThis.window = originalWindow
  })

  it('should filter mutations by type and target', async () => {
    // const { createThemeStore } = await import('../theme-store')
    const themeStore = createServerThemeStore()

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
})
