import { getTheme } from '../get-theme'

describe('getTheme', () => {
  it('should return theme', async () => {
    document.documentElement.setAttribute('data-theme', 'dark')
    expect(getTheme()).toBe('dark')
  })
})
