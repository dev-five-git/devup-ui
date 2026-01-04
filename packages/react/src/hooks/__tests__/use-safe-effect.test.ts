import { afterAll, describe, expect, it } from 'bun:test'
import { useEffect, useLayoutEffect } from 'react'

describe('useSafeEffect', () => {
  const originalWindow = globalThis.window

  afterAll(() => {
    globalThis.window = originalWindow
    // Clear module cache
    Loader.registry.delete(require.resolve('../use-safe-effect'))
  })

  it('should return useEffect when window is undefined (server)', async () => {
    // @ts-expect-error - intentionally setting window to undefined
    globalThis.window = undefined
    Loader.registry.delete(require.resolve('../use-safe-effect'))

    const { useSafeEffect } = await import('../use-safe-effect')
    expect(useSafeEffect).toBe(useEffect)
  })

  it('should return useLayoutEffect when window is defined (client)', async () => {
    // @ts-expect-error - intentionally setting window to object
    globalThis.window = {}
    Loader.registry.delete(require.resolve('../use-safe-effect'))

    const { useSafeEffect } = await import('../use-safe-effect')
    expect(useSafeEffect).toBe(useLayoutEffect)
  })
})
