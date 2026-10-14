import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { ThemeProvider } from '../../components/ThemeProvider'
import { useStyledTheme } from '../use-styled-theme'

function Read() {
  return <span>{JSON.stringify(useStyledTheme())}</span>
}

describe('useStyledTheme', () => {
  it('returns the theme object the nearest provider gives', () => {
    const { container } = render(
      <ThemeProvider theme={{ colors: { brand: 'red' } }}>
        <Read />
      </ThemeProvider>,
    )
    expect(container.textContent).toBe('{"colors":{"brand":"red"}}')
  })

  it('returns an empty object without a provider', () => {
    const { container } = render(<Read />)
    expect(container.textContent).toBe('{}')
  })
})
