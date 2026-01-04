import { afterAll, beforeAll, describe, expect, it } from 'bun:test'

import type { DevupTheme } from '../../types/theme'
import { setTheme } from '../set-theme'

beforeAll(() => {
  document.documentElement.removeAttribute('data-theme')
})

afterAll(() => {
  document.documentElement.removeAttribute('data-theme')
})

describe('setTheme', () => {
  it('should set theme', async () => {
    expect(document.documentElement.getAttribute('data-theme')).toBe(null)
    setTheme('dark' as keyof DevupTheme)
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })
})
