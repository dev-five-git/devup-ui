import { beforeEach, describe, expect, it, mock } from 'bun:test'
import { act, render, renderHook } from 'bun-test-env-dom'
import { createElement, type ReactNode } from 'react'

/** The system appearance React Native reports, and who listens to it */
const appearance = {
  scheme: 'light' as string | null,
  listeners: [] as (() => void)[],
}

mock.module('react-native', () => ({
  Appearance: {
    getColorScheme: () => appearance.scheme,
    addChangeListener: (listener: () => void) => {
      appearance.listeners.push(listener)
      return { remove: () => {} }
    },
  },
}))

/** The system appearance changing to `scheme` */
function changeAppearance(scheme: string | null) {
  act(() => {
    appearance.scheme = scheme
    for (const listener of appearance.listeners) listener()
  })
}

const runtime = await import('../index')
const { defineThemes, getTheme, setTheme, Themed, useTheme, useThemeIndex } =
  runtime

beforeEach(() => {
  appearance.scheme = 'light'
  defineThemes(['light', 'dark', 'sepia'], 1)
  setTheme(null)
})

describe('getTheme', () => {
  it('follows the system appearance until a theme is set', () => {
    expect(getTheme() as string | null).toBe('light')
    appearance.scheme = 'dark'
    expect(getTheme() as string | null).toBe('dark')
    setTheme('sepia' as Parameters<typeof setTheme>[0])
    expect(getTheme() as string | null).toBe('sepia')
    setTheme(null)
    expect(getTheme() as string | null).toBe('dark')
  })

  it('shows the default theme in the dark when no theme is dark', () => {
    defineThemes(['default'])
    appearance.scheme = 'dark'
    expect(getTheme() as string | null).toBe('default')
  })

  it('is null without themes', () => {
    defineThemes([])
    expect(getTheme()).toBeNull()
  })
})

describe('useTheme and useThemeIndex', () => {
  it('render again when the system appearance changes', () => {
    const { result } = renderHook(() => [useTheme(), useThemeIndex()])
    expect(result.current as unknown[]).toEqual(['light', 0])
    changeAppearance('dark')
    expect(result.current as unknown[]).toEqual(['dark', 1])
    changeAppearance(null)
    expect(result.current as unknown[]).toEqual(['light', 0])
  })

  it('render again when a theme is set, which wins over the system', () => {
    const { result, unmount } = renderHook(() => [useTheme(), useThemeIndex()])
    act(() => setTheme('sepia' as Parameters<typeof setTheme>[0]))
    expect(result.current as unknown[]).toEqual(['sepia', 2])
    changeAppearance('dark')
    expect(result.current as unknown[]).toEqual(['sepia', 2])
    unmount()
  })

  it('show the default theme for a name the themes lack', () => {
    const { result } = renderHook(() => [useTheme(), useThemeIndex()])
    act(() => setTheme('missing' as Parameters<typeof setTheme>[0]))
    expect(result.current as unknown[]).toEqual(['missing', 0])
  })

  it('give the first index without themes', () => {
    defineThemes([])
    const { result } = renderHook(() => [useTheme(), useThemeIndex()])
    expect(result.current as unknown[]).toEqual([null, 0])
  })

  it('listen to the system appearance once', () => {
    renderHook(() => useTheme())
    renderHook(() => useTheme())
    expect(appearance.listeners).toHaveLength(1)
  })
})

describe('Themed', () => {
  it('renders the component with the style of the theme that shows', () => {
    const styles = [{ color: 'black' }, { color: 'white' }, { color: 'brown' }]
    const Probe = ({
      style,
      children,
    }: {
      style?: { color: string }
      children?: ReactNode
    }) => createElement('span', { 'data-color': style?.color }, children)
    const { container } = render(
      createElement(
        Themed,
        { $c: Probe, $s: (theme: number) => styles[theme] },
        'text',
      ),
    )
    const span = () => container.querySelector('span')
    expect(span()?.getAttribute('data-color')).toBe('black')
    expect(span()?.textContent).toBe('text')
    changeAppearance('dark')
    expect(span()?.getAttribute('data-color')).toBe('white')
    act(() => setTheme('sepia' as Parameters<typeof setTheme>[0]))
    expect(span()?.getAttribute('data-color')).toBe('brown')
  })
})
