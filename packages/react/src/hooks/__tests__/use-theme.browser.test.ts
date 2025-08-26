import { renderHook, waitFor } from '@testing-library/react'

beforeEach(() => {
  vi.resetModules()
  vi.resetAllMocks()
})

describe('useTheme', () => {
  it('should return theme', async () => {
    const { useTheme } = await import('../use-theme')
    const { result, unmount } = renderHook(() => useTheme())
    expect(result.current).toBeNull()

    document.documentElement.setAttribute('data-theme', 'dark')
    await waitFor(() => {
      expect(result.current).toBe('dark')
    })
    const { result: newResult, unmount: newUnmount } = renderHook(() =>
      useTheme(),
    )
    expect(newResult.current).toBe('dark')
    newUnmount()
    unmount()
  })

  it('should return theme when already set', async () => {
    const { useTheme } = await import('../use-theme')
    document.documentElement.setAttribute('data-theme', 'dark')
    const { result, unmount } = renderHook(() => useTheme())
    expect(result.current).toBe('dark')
    unmount()
  })
})
