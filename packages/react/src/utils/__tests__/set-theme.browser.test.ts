import type { DevupTheme } from '../../types/theme'
import { setTheme } from '../set-theme'

describe('setTheme', () => {
  it('should set theme', async () => {
    expect(document.documentElement.getAttribute('data-theme')).toBe(null)
    setTheme('dark' as keyof DevupTheme)
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })
})
