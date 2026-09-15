import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { ThemeProvider } from '../ThemeProvider'

describe('ThemeProvider', () => {
  it('should declare css variables without affecting layout', () => {
    const { container } = render(
      <ThemeProvider theme={{ brand: 'red', colors: { accent: 'blue' } }}>
        <span>child</span>
      </ThemeProvider>,
    )
    expect(container).toMatchSnapshot()
  })

  it('should render without a theme', () => {
    const { container } = render(
      <ThemeProvider>
        <span>child</span>
      </ThemeProvider>,
    )
    expect(container).toMatchSnapshot()
  })
})
