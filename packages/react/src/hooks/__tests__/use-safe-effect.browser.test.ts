import { useLayoutEffect } from 'react'

import { useSafeEffect } from '../use-safe-effect'

describe('useSafeEffect', () => {
  it('should return useLayoutEffect', async () => {
    expect(useSafeEffect).toBe(useLayoutEffect)
  })
})
