import { afterAll, beforeAll, describe, expect, it } from 'bun:test'
import { act, renderHook } from 'bun-test-env-dom'

beforeAll(() => {
  document.documentElement.removeAttribute('data-theme')
})

afterAll(() => {
  document.documentElement.removeAttribute('data-theme')
})

// `use-theme` builds its store at evaluation time, so this file imports it
// through a unique query string to get an instance created after the attribute
// reset above. `Loader.registry`, the previous way to evict it, was removed in
// Bun 1.4.
const importUseTheme = () => import('../use-theme?browser-test')

// Helper to wait for MutationObserver to process
const waitForMutationObserver = () =>
  new Promise((resolve) => setTimeout(resolve, 10))

describe('useTheme', () => {
  it('should return theme', async () => {
    const { useTheme } = await importUseTheme()
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
    const { useTheme } = await importUseTheme()
    const { result, unmount } = renderHook(() => useTheme())
    expect(result.current).toBe('dark')
    unmount()
  })
})
