import { renderHook, waitFor } from '@testing-library/react'

beforeEach(() => {
  vi.resetModules()
})

describe('useTheme', () => {
  it('should return theme', async () => {
    const { useTheme } = await import('../use-theme')
    const { result } = renderHook(() => useTheme())
    expect(result.current).toBeNull()

    document.documentElement.setAttribute('data-theme', 'dark')
    await waitFor(() => {
      expect(result.current).toBe('dark')
    })
    const { result: newResult } = renderHook(() => useTheme())
    expect(newResult.current).toBe('dark')
  })
  it('should return theme when already set', async () => {
    const { useTheme } = await import('../use-theme')
    document.documentElement.setAttribute('data-theme', 'dark')
    const { result } = renderHook(() => useTheme())
    expect(result.current).toBe('dark')
  })
})
