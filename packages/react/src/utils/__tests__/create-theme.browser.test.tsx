import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { ThemeProvider } from '../../components/ThemeProvider'
import { createTheme } from '../create-theme'

describe('createTheme', () => {
  const theme = createTheme({ colors: { primary: '#0070f3' }, space: 4 })

  it('reads each leaf as a variable with its value as fallback', () => {
    expect(theme.colors.primary).toBe('var(--sc-colors-primary, #0070f3)')
    expect(theme.space).toBe('var(--sc-space, 4)')
    expect(theme.vars.colors.primary).toBe('--sc-colors-primary')
    expect(theme.raw.space).toBe(4)
  })

  it('declares the variables from the provider theme', () => {
    const custom = createTheme(
      { colors: { primary: 'red' }, space: 4 },
      { prefix: 'ds', selector: ':host' },
    )
    const { container } = render(
      <ThemeProvider theme={{ colors: { primary: 'blue' }, space: { n: 1 } }}>
        <custom.GlobalStyle />
      </ThemeProvider>,
    )
    expect(container.querySelector('style')?.textContent).toBe(
      ':host{--ds-colors-primary:blue;}',
    )
    const { container: bare } = render(<theme.GlobalStyle />)
    expect(bare.querySelector('style')?.textContent).toBe(':root{}')
    const { container: flat } = render(
      <ThemeProvider theme={{ colors: 'x', space: 8 }}>
        <theme.GlobalStyle />
      </ThemeProvider>,
    )
    expect(flat.querySelector('style')?.textContent).toBe(
      ':root{--sc-space:8;}',
    )
  })

  it('resolves the declared values in the browser', () => {
    document.documentElement.style.setProperty('--sc-space', '12')
    expect(theme.resolve()).toEqual({
      colors: { primary: '#0070f3' },
      space: '12',
    })
    document.documentElement.style.removeProperty('--sc-space')
    expect(theme.resolve(document.body).space).toBe(4)
  })
})
