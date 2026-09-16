import { describe, expect, it } from 'bun:test'

import { useStyledTheme } from '../use-styled-theme'

describe('useStyledTheme', () => {
  it('returns css variable references rather than a theme name', () => {
    const theme = useStyledTheme<{ colors: { brand: string } }>()
    expect(`${theme.colors.brand}`).toBe('var(--colors-brand)')
  })
})
