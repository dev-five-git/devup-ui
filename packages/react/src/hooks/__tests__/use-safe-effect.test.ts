import { afterAll, describe, expect, it } from 'bun:test'
import { useEffect, useLayoutEffect } from 'react'

describe('useSafeEffect', () => {
  const originalWindow = globalThis.window

  afterAll(() => {
    globalThis.window = originalWindow
  })

  // The module picks its effect at evaluation time, so each case needs a fresh
  // evaluation. A unique query string gives every import its own cache entry -
  // `Loader.registry`, the previous way to evict it, was removed in Bun 1.4.
  it('should return useEffect when window is undefined (server)', async () => {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: undefined,
    })

    const { useSafeEffect } = await import('../use-safe-effect?server')
    expect(useSafeEffect).toBe(useEffect)
  })

  it('should return useLayoutEffect when window is defined (client)', async () => {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {},
    })

    const { useSafeEffect } = await import('../use-safe-effect?client')
    expect(useSafeEffect).toBe(useLayoutEffect)
  })
})
