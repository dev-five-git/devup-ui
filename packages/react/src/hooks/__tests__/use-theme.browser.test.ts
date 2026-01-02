import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import { act, renderHook } from 'bun-test-env-dom'

beforeEach(() => {
  document.documentElement.removeAttribute('data-theme')
  // Clear module caches for fresh state
  Loader.registry.delete(require.resolve('../use-theme'))
  Loader.registry.delete(require.resolve('../../stores/theme-store'))
})

afterEach(() => {
  document.documentElement.removeAttribute('data-theme')
})

// Helper to wait for MutationObserver to process
const waitForMutationObserver = () =>
  new Promise((resolve) => setTimeout(resolve, 10))

describe('useTheme', () => {
  it('should return theme', async () => {
    const { useTheme } = await import('../use-theme')
    const { result, unmount } = renderHook(() => useTheme())
    expect(result.current).toBeNull()

    await act(async () => {
      document.documentElement.setAttribute('data-theme', 'dark')
      await waitForMutationObserver()
    })
    expect(result.current as string | null).toBe('dark')

    const { result: newResult, unmount: newUnmount } = renderHook(() =>
      useTheme(),
    )
    expect(newResult.current as string | null).toBe('dark')
    newUnmount()
    unmount()
  })

  it('should return theme when already set', async () => {
    document.documentElement.setAttribute('data-theme', 'dark')
    const { useTheme } = await import('../use-theme')
    const { result, unmount } = renderHook(() => useTheme())
    expect(result.current).toBe('dark')
    unmount()
  })
})
