import { initTheme } from '../init-theme'

describe('initTheme', () => {
  it('should initialize the theme', () => {
    initTheme()
    expect(document.documentElement.getAttribute('data-theme')).toBe('default')
  })
  it('should initialize the theme with the given theme', () => {
    initTheme(false, 'light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })

  it('should initialize the theme with the default theme', () => {
    vi.spyOn(window, 'matchMedia').mockReturnValue({
      matches: true,
    } as MediaQueryList)

    initTheme(true)
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })
})
