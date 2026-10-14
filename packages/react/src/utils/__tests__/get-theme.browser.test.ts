import { describe, expect, it } from 'bun:test'

import { getTheme } from '../get-theme'

describe('getTheme', () => {
  it('should return theme', async () => {
    const originalTheme = document.documentElement.getAttribute('data-theme')
    try {
      document.documentElement.setAttribute('data-theme', 'dark')
      expect(getTheme()).toBe('dark')
    } finally {
      if (originalTheme === null) {
        document.documentElement.removeAttribute('data-theme')
      } else {
        document.documentElement.setAttribute('data-theme', originalTheme)
      }
    }
  })
})
