import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import { initTheme } from '../init-theme'

let originalTheme: string | null
let originalSelection: string | null

beforeEach(() => {
  originalTheme = document.documentElement.getAttribute('data-theme')
  originalSelection = localStorage.getItem('__DF_THEME_SELECTED__')
  localStorage.removeItem('__DF_THEME_SELECTED__')
})

afterEach(() => {
  if (originalTheme === null) {
    document.documentElement.removeAttribute('data-theme')
  } else {
    document.documentElement.setAttribute('data-theme', originalTheme)
  }
  if (originalSelection === null) {
    localStorage.removeItem('__DF_THEME_SELECTED__')
  } else {
    localStorage.setItem('__DF_THEME_SELECTED__', originalSelection)
  }
})

describe('initTheme', () => {
  it('should initialize the theme', () => {
    const matchMediaSpy = spyOn(window, 'matchMedia').mockReturnValue({
      matches: false,
    } as MediaQueryList)
    try {
      initTheme()
      expect(document.documentElement.getAttribute('data-theme')).toBe(
        'default',
      )
    } finally {
      matchMediaSpy.mockRestore()
    }
  })
  it('should initialize the theme with the given theme', () => {
    initTheme(false, 'light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })

  it('should initialize the theme with the default theme', () => {
    const matchMediaSpy = spyOn(window, 'matchMedia').mockReturnValue({
      matches: true,
    } as MediaQueryList)
    try {
      initTheme(true)
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    } finally {
      matchMediaSpy.mockRestore()
    }
  })
})
