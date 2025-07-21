import { useEffect, useLayoutEffect } from 'react'

beforeEach(() => {
  vi.resetModules()
})
describe('useSafeEffect', () => {
  it('return useEffect in the server', async () => {
    const { useSafeEffect } = await import('../use-safe-effect')
    // @ts-ignore
    expect(useSafeEffect).toBe(useEffect)
  })
  it('return useEffect in the client', async () => {
    // @ts-ignore
    globalThis.window = {}

    const { useSafeEffect } = await import('../use-safe-effect')
    expect(useSafeEffect).toBe(useLayoutEffect)
  })
})
