import {
  createElement,
  type ElementType,
  type ReactElement,
  useSyncExternalStore,
} from 'react'
import { Appearance } from 'react-native'

import type { DevupTheme } from '../types/theme'

/**
 * The React Native runtime of Devup UI. The build compiles every style to
 * `StyleSheet` objects, one a theme when a value differs by theme; this
 * module only tells which theme shows. It reads no CSS, resolves no token
 * and computes no style. Web builds never import it.
 */

type ThemeName = keyof DevupTheme extends undefined ? string : keyof DevupTheme

/** The theme names, in the order the build indexes style objects by */
let names: readonly string[] = []
/** The index of the theme the system's dark appearance shows */
let dark = -1
/** The theme `setTheme` chose, which wins over the system appearance */
let chosen: string | null = null
let following = false
const listeners = new Set<() => void>()

function notify(): void {
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  if (!following) {
    following = true
    Appearance.addChangeListener(notify)
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
 * Renders `$c` with the style `$s` gives for the theme the app shows. The
 * build renders an element through it where no function component can read
 * the theme for it.
 * @internal
 */
export function Themed({
  $c,
  $s,
  ...props
}: {
  $c: ElementType
  $s: (theme: number) => unknown
} & Record<string, unknown>): ReactElement {
  return createElement($c, { ...props, style: $s(useThemeIndex()) })
}
