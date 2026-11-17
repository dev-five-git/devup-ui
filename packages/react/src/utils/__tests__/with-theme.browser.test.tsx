import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'
import { createRef, forwardRef } from 'react'

import { ThemeProvider } from '../../components/ThemeProvider'
import type { StyledTheme } from '../theme-vars'
import { hoistStatics, ThemeConsumer, withTheme } from '../with-theme'

const Swatch = forwardRef<HTMLDivElement, { theme?: StyledTheme }>(
  function Swatch({ theme }, ref) {
    return <div ref={ref}>{String(theme?.brand)}</div>
  },
)
Swatch.displayName = 'Swatch'

describe('withTheme', () => {
  it('injects the theme the nearest provider gives and forwards refs', () => {
    const Themed = withTheme(Swatch)
    const ref = createRef<HTMLDivElement>()
    const { container } = render(
      <ThemeProvider theme={{ brand: 'red' }}>
        <Themed ref={ref} />
      </ThemeProvider>,
    )
    expect(container.textContent).toBe('red')
    expect(ref.current?.tagName).toBe('DIV')
    expect(Themed.displayName).toBe('WithTheme(Swatch)')
  })

  it('lets the caller theme win and falls back to an empty theme', () => {
    const Themed = withTheme(Swatch)
    expect(
      render(<Themed theme={{ brand: 'blue' }} />).container.textContent,
    ).toBe('blue')
    expect(render(<Themed />).container.textContent).toBe('undefined')
  })

  it('hoists the statics that are not React’s', () => {
    const Item = () => null
    const symbol = Symbol('static')
    const List = Object.assign(() => null, {
      Item,
      [symbol]: 1,
      propTypes: {},
      defaultProps: { theme: {} },
      displayName: 'List',
    })
    Object.defineProperty(List, 'hidden', { value: 2, enumerable: false })
    const Themed = withTheme(List)
    expect(Reflect.get(Themed, 'Item')).toBe(Item)
    expect(Reflect.get(Themed, symbol)).toBe(1)
    expect(Reflect.get(Themed, 'hidden')).toBe(2)
    expect(Reflect.get(Themed, 'propTypes')).toBeUndefined()
    expect(Reflect.get(Themed, 'defaultProps')).toBeUndefined()
    expect(Themed.displayName).toBe('WithTheme(List)')
  })

  it('keeps what the wrapper defines itself', () => {
    const target = { render: 'own', own: 'target' }
    expect(hoistStatics(target, { own: 'source' })).toEqual({
      render: 'own',
      own: 'target',
    })
  })

  it('names anonymous components', () => {
    const Anonymous = withTheme(() => null)
    expect(Anonymous.displayName).toBe('WithTheme(Component)')
    const Named = Object.assign(() => null, { displayName: 'Named' })
    expect(withTheme(Named).displayName).toBe('WithTheme(Named)')
  })
})

describe('ThemeConsumer', () => {
  it('renders what children make of the theme', () => {
    const { container } = render(
      <ThemeProvider theme={{ brand: 'green' }}>
        <ThemeConsumer>{(theme) => String(theme.brand)}</ThemeConsumer>
      </ThemeProvider>,
    )
    expect(container.textContent).toBe('green')
    expect(
      render(<ThemeConsumer>{(theme) => JSON.stringify(theme)}</ThemeConsumer>)
        .container.textContent,
    ).toBe('{}')
  })
})
