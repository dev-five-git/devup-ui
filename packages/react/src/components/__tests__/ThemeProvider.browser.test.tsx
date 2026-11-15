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
    expect(
      [...container.querySelectorAll('span')].map((span) => span.textContent),
    ).toEqual([
      '{"a":"outer","b":"inner"}',
      '{"a":"outer","b":"outer","c":"fn"}',
    ])
  })
})

describe('ThemeProvider on :root', () => {
  const rootRules = (container: HTMLElement) =>
    [...container.querySelectorAll('style')].map((style) => style.textContent)

  it('declares the theme of the outermost provider on :root only', () => {
    const { container } = render(
      <ThemeProvider theme={{ brand: 'red', colors: { accent: 'blue' } }}>
        <ThemeProvider theme={{ brand: 'green' }}>
          <span>child</span>
        </ThemeProvider>
      </ThemeProvider>,
    )
    expect(rootRules(container)).toEqual([
      ':root{--brand:red;--colors-accent:blue}',
    ])
  })

  it('declares nothing on :root without a theme', () => {
    const { container } = render(
      <ThemeProvider>
        <span>child</span>
      </ThemeProvider>,
    )
    expect(rootRules(container)).toEqual([])
  })

  it('follows a theme change and leaves with the provider', () => {
    const { container, rerender, unmount } = render(
      <ThemeProvider theme={{ brand: 'red' }}>
        <span>child</span>
      </ThemeProvider>,
    )
    const child = container.querySelector('span')
    rerender(
      <ThemeProvider theme={{ brand: 'black' }}>
        <span>child</span>
      </ThemeProvider>,
    )
    expect(rootRules(container)).toEqual([':root{--brand:black}'])
    expect(container.querySelector('span')).toBe(child)
    unmount()
    expect(document.querySelectorAll('style')).toHaveLength(0)
  })

  it('keeps a value from closing its style element', () => {
    const { container } = render(
      <ThemeProvider theme={{ brand: '</style><b>' }}>
        <span>child</span>
      </ThemeProvider>,
    )
    expect(rootRules(container)).toEqual([':root{--brand:\\3c /style>\\3c b>}'])
  })

  it('gives global styles on body the theme of the outermost provider', () => {
    const { unmount } = render(
      <ThemeProvider theme={{ colors: { page: 'rgb(1, 2, 3)' } }}>
        <span>child</span>
      </ThemeProvider>,
    )
    const global = document.createElement('style')
    global.textContent = 'body{color:var(--colors-page)}'
    document.head.append(global)
    try {
      expect(getComputedStyle(document.body).color).toBe('rgb(1, 2, 3)')
    } finally {
      global.remove()
      unmount()
    }
  })
})

describe('resolveTheme', () => {
  it('keeps the outer theme without its own', () => {
    expect(resolveTheme(undefined, { a: 1 })).toEqual({ a: 1 })
    expect(resolveTheme({ a: 1 }, undefined)).toEqual({ a: 1 })
    expect(resolveTheme(undefined, undefined)).toBeUndefined()
  })
})
