import { beforeEach } from 'node:test'

import { describe, expect, it, mock, spyOn } from 'bun:test'
import { afterAll } from 'bun:test'

import { initTheme } from '../init-theme'

afterAll(() => {
  mock.restore()
})
beforeEach(() => {
  localStorage.removeItem('__DF_THEME_SELECTED__')
})

describe('initTheme', () => {
  it('should initialize the theme', () => {
    spyOn(window, 'matchMedia').mockReturnValue({
      matches: false,
    } as MediaQueryList)
    initTheme()
    expect(document.documentElement.getAttribute('data-theme')).toBe('default')
  })
  it('should initialize the theme with the given theme', () => {
    initTheme(false, 'light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })

  it('should initialize the theme with the default theme', () => {
    spyOn(window, 'matchMedia').mockReturnValue({
      matches: true,
    } as MediaQueryList)

    initTheme(true)
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })
})
