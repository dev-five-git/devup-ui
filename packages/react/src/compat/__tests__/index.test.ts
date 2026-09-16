import { describe, expect, it } from 'bun:test'

describe('compat entry', () => {
  it('exports only the absorbed third-party APIs', async () => {
    const compat = await import('../index')

    expect({ ...compat }).toEqual({
      Global: expect.any(Function),
      ThemeProvider: expect.any(Function),

      createGlobalStyle: expect.any(Function),
      useTheme: expect.any(Function),
      withTheme: expect.any(Function),

      isStyledComponent: expect.any(Function),
      ServerStyleSheet: expect.any(Function),
      StyleSheetManager: expect.any(Function),
    })
  })

  it('keeps its useTheme distinct from the devup-ui one', async () => {
    const { useTheme: compatUseTheme } = await import('../index')
    const { useTheme: devupUseTheme } = await import('../../index')

    expect(compatUseTheme).not.toBe(devupUseTheme)
    expect(`${compatUseTheme<{ brand: string }>().brand}`).toBe('var(--brand)')
  })
})
