import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { useStyledTheme } from '../../hooks/use-styled-theme'
import { resolveTheme, ThemeProvider } from '../ThemeProvider'

function Read() {
  return <span>{JSON.stringify(useStyledTheme())}</span>
}

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

  it('merges a nested theme object and calls a nested theme function', () => {
    const { container } = render(
      <ThemeProvider theme={{ a: 'outer', b: 'outer' }}>
        <ThemeProvider theme={{ b: 'inner' }}>
          <Read />
        </ThemeProvider>
        <ThemeProvider theme={(outer) => ({ ...outer, c: 'fn' })}>
          <Read />
        </ThemeProvider>
      </ThemeProvider>,
    )
    expect(container.textContent).toBe(
      '{"a":"outer","b":"inner"}{"a":"outer","b":"outer","c":"fn"}',
    )
  })
})

describe('resolveTheme', () => {
  it('keeps the outer theme without its own', () => {
    expect(resolveTheme(undefined, { a: 1 })).toEqual({ a: 1 })
    expect(resolveTheme({ a: 1 }, undefined)).toEqual({ a: 1 })
    expect(resolveTheme(undefined, undefined)).toBeUndefined()
  })
})
