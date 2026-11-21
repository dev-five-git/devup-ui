import { beforeEach, describe, expect, it, mock } from 'bun:test'
import { act, render, renderHook } from 'bun-test-env-dom'
import { createElement, type ReactNode } from 'react'

/** The system appearance React Native reports, and who listens to it */
const appearance = {
  scheme: 'light' as string | null,
  listeners: [] as (() => void)[],
}

/** The window React Native reports, and who listens to its changes */
const dimensions = {
  width: 390,
  height: 844,
  listeners: [] as (() => void)[],
}

mock.module('react-native', () => ({
  Dimensions: {
    get: () => ({ width: dimensions.width, height: dimensions.height }),
    addEventListener: (event: string, listener: () => void) => {
      if (event === 'change') dimensions.listeners.push(listener)
      return { remove: () => {} }
    },
  },
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

/** The window resizing to `width` by `height` */
function resize(width: number, height: number) {
  act(() => {
    dimensions.width = width
    dimensions.height = height
    for (const listener of dimensions.listeners) listener()
  })
}

const runtime = await import('../index')
const {
  defineBreakpoints,
  defineThemes,
  getBreakpoint,
  getTheme,
  setTheme,
  Themed,
  useBreakpoint,
  useLandscape,
  useTheme,
  useThemeIndex,
} = runtime

beforeEach(() => {
  appearance.scheme = 'light'
  dimensions.width = 390
  dimensions.height = 844
  defineBreakpoints([0, 480, 768, 992, 1280, 1600])
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

describe('getBreakpoint and useBreakpoint', () => {
  it('give the last breakpoint the window width reaches', () => {
    expect(getBreakpoint()).toBe(0)
    dimensions.width = 480
    expect(getBreakpoint()).toBe(1)
    dimensions.width = 1599
    expect(getBreakpoint()).toBe(4)
    dimensions.width = 4000
    expect(getBreakpoint()).toBe(5)
  })

  it('read the breakpoints devup.json gives', () => {
    defineBreakpoints([100, 600])
    dimensions.width = 50
    expect(getBreakpoint()).toBe(0)
    dimensions.width = 599
    expect(getBreakpoint()).toBe(0)
    dimensions.width = 600
    expect(getBreakpoint()).toBe(1)
    defineBreakpoints([])
    expect(getBreakpoint()).toBe(0)
  })

  it('render again only when the window crosses a breakpoint', () => {
    let renders = 0
    const { result } = renderHook(() => {
      renders += 1
      return useBreakpoint()
    })
    expect([result.current, renders]).toEqual([0, 1])
    resize(400, 844)
    resize(479, 844)
    expect([result.current, renders]).toEqual([0, 1])
    resize(480, 844)
    expect([result.current, renders]).toEqual([1, 2])
    resize(700, 844)
    expect([result.current, renders]).toEqual([1, 2])
    resize(1024, 844)
    expect(result.current).toBe(3)
    resize(320, 844)
    expect(result.current).toBe(0)
  })
})

describe('useLandscape', () => {
  it('renders again when the window turns', () => {
    const { result } = renderHook(() => useLandscape())
    expect(result.current).toBe(false)
    resize(844, 390)
    expect(result.current).toBe(true)
    resize(390, 844)
    expect(result.current).toBe(false)
  })

  it('shares one listener of the window with every hook', () => {
    renderHook(() => [useLandscape(), useBreakpoint(), useTheme()])
    renderHook(() => useBreakpoint())
    expect(dimensions.listeners).toHaveLength(1)
  })
})

describe('Themed with breakpoints', () => {
  it('renders the style of the breakpoint and orientation of the window', () => {
    const Probe = ({ style }: { style?: string }) =>
      createElement('span', { 'data-style': style })
    const { container } = render(
      createElement(Themed, {
        $c: Probe,
        $s: (theme: number, breakpoint: number, landscape: boolean) =>
          `${theme}-${breakpoint}-${landscape}`,
      }),
    )
    const style = () =>
      container.querySelector('span')?.getAttribute('data-style')
    expect(style()).toBe('0-0-false')
    resize(844, 390)
    expect(style()).toBe('0-2-true')
    changeAppearance('dark')
    expect(style()).toBe('1-2-true')
  })
})

describe('Focused', () => {
  it('renders the component with the style for whether it has focus', () => {
    const calls: string[] = []
    const Probe = ({
      style,
      onFocus,
      onBlur,
    }: {
      style?: { color: string }
      onFocus: (event: unknown) => void
      onBlur: (event: unknown) => void
    }) =>
      createElement('input', {
        'data-color': style?.color,
        onFocus: () => onFocus('focus'),
        onBlur: () => onBlur('blur'),
      })
    const { container, rerender } = render(
      createElement(runtime.Focused, {
        $c: Probe,
        $s: (focused: boolean, theme: number) => ({
          color: `${focused ? 'focused' : 'idle'} ${theme}`,
        }),
        onFocus: (event: unknown) => calls.push(String(event)),
        onBlur: (event: unknown) => calls.push(String(event)),
      }),
    )
    const input = () => container.querySelector('input')
    expect(input()?.getAttribute('data-color')).toBe('idle 0')
    act(() => input()?.focus())
    expect(input()?.getAttribute('data-color')).toBe('focused 0')
    act(() => input()?.blur())
    expect(input()?.getAttribute('data-color')).toBe('idle 0')
    expect(calls).toEqual(['focus', 'blur'])
    rerender(
      createElement(runtime.Focused, {
        $c: Probe,
        $s: (focused: boolean) => ({ color: String(focused) }),
      }),
    )
    act(() => input()?.focus())
    act(() => input()?.blur())
    expect(input()?.getAttribute('data-color')).toBe('false')
  })
})
