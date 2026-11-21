import {
  createElement,
  type ElementType,
  type ReactElement,
  type ReactNode,
  useContext,
  useMemo,
  useState,
  useSyncExternalStore,
} from 'react'
import { AccessibilityInfo, Appearance, Dimensions } from 'react-native'

import type { StyledTheme } from '../types/props'
import type { DevupTheme } from '../types/theme'
import { StyledThemeContext } from '../utils/theme-context'

/**
 * The React Native runtime of Devup UI. The build compiles every style to
 * `StyleSheet` objects, one a theme or a breakpoint when a value differs by
 * them; this module only tells which theme shows, which breakpoint the
 * window is at, whether it is in landscape, whether the system reduces
 * motion and, for viewport lengths, the window's size. It reads no CSS and
 * resolves no token. Web builds never import it.
 */

type ThemeName = keyof DevupTheme extends undefined ? string : keyof DevupTheme

/** The theme names, in the order the build indexes style objects by */
let names: readonly string[] = []
/** The index of the theme the system's dark appearance shows */
let dark = -1
/** The theme `setTheme` chose, which wins over the system appearance */
let chosen: string | null = null
/** The window widths the breakpoints start at, as devup.json gives them */
let breakpoints: readonly number[] = [0, 480, 768, 992, 1280, 1600]
/** Whether the system asks apps to reduce motion */
let reduced = false
let following = false
const listeners = new Set<() => void>()

/**
 * Tells every listener that something changed. Each hook reads only its own
 * value, and React renders a component again only when the value its hook
 * reads changes, so a resize that crosses no breakpoint renders again only
 * the components reading the window's size.
 */
function notify(): void {
  for (const listener of listeners) listener()
}

function setReduced(value: boolean): void {
  reduced = value
  notify()
}

function subscribe(listener: () => void): () => void {
  if (!following) {
    following = true
    Appearance.addChangeListener(notify)
    Dimensions.addEventListener('change', notify)
    AccessibilityInfo.addEventListener('reduceMotionChanged', setReduced)
    void AccessibilityInfo.isReduceMotionEnabled().then(setReduced)
  }
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/**
 * Tells the runtime the theme names of devup.json, the default first, and
 * the index of the one the system's dark appearance shows. The build calls
 * it in each file that uses the runtime.
 * @internal
 */
export function defineThemes(themes: readonly string[], darkIndex = -1): void {
  names = themes
  dark = darkIndex
}

/**
 * Tells the runtime the breakpoints of devup.json when they are not the
 * web's default. The build calls it in each file that reads the breakpoint.
 * @internal
 */
export function defineBreakpoints(widths: readonly number[]): void {
  breakpoints = widths
}

/**
 * The theme the app shows: the one `setTheme` chose, else the dark theme
 * while the system appearance is dark, else the default theme; `null`
 * without themes
 */
export function getTheme(): ThemeName | null {
  const system =
    Appearance.getColorScheme() === 'dark' ? names[dark] : undefined
  return (chosen ?? system ?? names[0] ?? null) as ThemeName | null
}

/**
 * Shows `theme` whatever the system appearance is, as `setTheme` does on the
 * web; `null` follows the system appearance again. A name devup.json does
 * not define shows the default theme's styles, as on the web. The choice is
 * not saved across launches.
 */
export function setTheme(theme: ThemeName | null): void {
  chosen = theme
  notify()
}

/** The theme the app shows, rendering again when it changes */
export function useTheme(): ThemeName | null {
  return useSyncExternalStore(subscribe, getTheme, getTheme)
}

/**
 * The index of the theme the app shows among the theme names, which the
 * build picks a style object by
 * @internal
 */
export function useThemeIndex(): number {
  return Math.max(names.indexOf(useTheme() ?? ''), 0)
}

/**
 * The breakpoint the window is at: the index of the last breakpoint whose
 * width the window reaches, as the web's `min-width` media queries choose,
 * and `0` below them all
 */
export function getBreakpoint(): number {
  const { width } = Dimensions.get('window')
  let level = 0
  for (let index = 1; index < breakpoints.length; index++) {
    if (width >= breakpoints[index]) level = index
  }
  return level
}

/**
 * The breakpoint the window is at, rendering again only when the window
 * crosses into another breakpoint
 */
export function useBreakpoint(): number {
  return useSyncExternalStore(subscribe, getBreakpoint, getBreakpoint)
}

/** Whether the window is wider than it is tall */
function getLandscape(): boolean {
  const { width, height } = Dimensions.get('window')
  return width > height
}

/**
 * Whether the window is in landscape, rendering again when it turns
 * @internal
 */
export function useLandscape(): boolean {
  return useSyncExternalStore(subscribe, getLandscape, getLandscape)
}

/** Whether the system asks apps to reduce motion */
function getReducedMotion(): boolean {
  return reduced
}

/**
 * Whether the system asks apps to reduce motion, which `_motionReduce` and
 * `_motionSafe` apply by, rendering again when the setting changes
 * @internal
 */
export function useReducedMotion(): boolean {
  return useSyncExternalStore(subscribe, getReducedMotion, getReducedMotion)
}

function getWindowWidth(): number {
  return Dimensions.get('window').width
}

function getWindowHeight(): number {
  return Dimensions.get('window').height
}

/**
 * The window's width, which a `vw` length is a hundredth of, rendering again
 * when it changes
 * @internal
 */
export function useWindowWidth(): number {
  return useSyncExternalStore(subscribe, getWindowWidth, getWindowWidth)
}

/**
 * The window's height, which a `vh` length is a hundredth of, rendering
 * again when it changes
 * @internal
 */
export function useWindowHeight(): number {
  return useSyncExternalStore(subscribe, getWindowHeight, getWindowHeight)
}

/** What `Themed` gives the style: what the runtime tells, in this order */
type Reads = (
  theme: number,
  breakpoint: number,
  landscape: boolean,
  reducedMotion: boolean,
) => unknown

/**
 * Renders `$c` with the style `$s` gives for the theme the app shows, the
 * breakpoint the window is at, whether it is in landscape and whether the
 * system reduces motion. The build renders an element through it where no
 * function component can read them for it.
 * @internal
 */
export function Themed({
  $c,
  $s,
  ...props
}: { $c: ElementType; $s: Reads } & Record<string, unknown>): ReactElement {
  return createElement($c, {
    ...props,
    style: $s(
      useThemeIndex(),
      useBreakpoint(),
      useLandscape(),
      useReducedMotion(),
    ),
  })
}

/**
 * `Themed` for an element that also takes a viewport length: its style also
 * takes the window's width and height. Only such elements render through it,
 * so a resize renders again only them.
 * @internal
 */
export function Sized({
  $c,
  $s,
  ...props
}: {
  $c: ElementType
  $s: (
    ...reads: [...Parameters<Reads>, width: number, height: number]
  ) => unknown
} & Record<string, unknown>): ReactElement {
  return createElement($c, {
    ...props,
    style: $s(
      useThemeIndex(),
      useBreakpoint(),
      useLandscape(),
      useReducedMotion(),
      useWindowWidth(),
      useWindowHeight(),
    ),
  })
}

/**
 * Renders `$c`, a `TextInput`, with the style `$s` gives for whether it has
 * focus, which a `TextInput` tells only through its events, and for the
 * theme, the breakpoint, the orientation and reduced motion, as `Themed`
 * does. The build renders an input taking a focus style with it.
 * @internal
 */
export function Focused({
  $c,
  $s,
  onFocus,
  onBlur,
  ...props
}: {
  $c: ElementType
  $s: (focused: boolean, ...reads: Parameters<Reads>) => unknown
  onFocus?: (event: unknown) => void
  onBlur?: (event: unknown) => void
} & Record<string, unknown>): ReactElement {
  const [focused, setFocused] = useState(false)
  return createElement($c, {
    ...props,
    onFocus: (event: unknown) => {
      setFocused(true)
      onFocus?.(event)
    },
    onBlur: (event: unknown) => {
      setFocused(false)
      onBlur?.(event)
    },
    style: $s(
      focused,
      useThemeIndex(),
      useBreakpoint(),
      useLandscape(),
      useReducedMotion(),
    ),
  })
}

export { withTheme } from '../utils/with-theme'

/**
 * The `ThemeProvider` of styled-components and Emotion, which the build
 * imports for theirs: it gives its descendants `theme`, merged over the
 * theme of an outer provider, or what a function of that theme returns. The
 * `styled()` components of those packages call their functions with it as
 * `props.theme`, and `withTheme` gives it as the `theme` prop.
 */
export function ThemeProvider({
  theme,
  children,
}: {
  theme: StyledTheme | ((outer: StyledTheme | undefined) => StyledTheme)
  children?: ReactNode
}): ReactElement {
  const outer = useContext(StyledThemeContext)
  const value = useMemo(
    () => (typeof theme === 'function' ? theme(outer) : { ...outer, ...theme }),
    [outer, theme],
  )
  return createElement(StyledThemeContext.Provider, { value }, children)
}

/**
 * The theme of the nearest `ThemeProvider`, or an empty object outside one:
 * the `useTheme` of styled-components and Emotion, which the build imports
 * for theirs
 */
export function useStyledTheme(): StyledTheme {
  return useContext(StyledThemeContext) ?? {}
}
