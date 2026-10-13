import { describe, expect, it } from 'bun:test'
import { createElement } from 'react'

describe('compat entry', () => {
  it('exports only the absorbed third-party APIs', async () => {
    const compat = await import('../index')

    expect({ ...compat }).toEqual({
      ClassNames: expect.any(Function),
      Global: expect.any(Function),
      ThemeProvider: expect.any(Function),

      createGlobalStyle: expect.any(Function),
      useTheme: expect.any(Function),
      withTheme: expect.any(Function),

      isStyledComponent: expect.any(Function),
      ServerStyleSheet: expect.any(Function),
      StyleSheetManager: expect.any(Function),

      jsx: expect.any(Function),
    })
  })

  it("builds Emotion's jsx elements as React does", async () => {
    const { jsx } = await import('../index')

    expect(jsx).toBe(createElement)
  })

  it('leaves ClassNames for the build to compile', async () => {
    const { ClassNames } = await import('../index')

    expect(() => ClassNames({ children: () => null })).toThrow(
      'Cannot run on the runtime',
    )
  })

  it('keeps its useTheme distinct from the devup-ui one', async () => {
    const { useTheme: compatUseTheme } = await import('../index')
    const { useTheme: devupUseTheme } = await import('../../index')

    expect(compatUseTheme).not.toBe(devupUseTheme)
    expect(`${compatUseTheme<{ brand: string }>().brand}`).toBe('var(--brand)')
  })
})
