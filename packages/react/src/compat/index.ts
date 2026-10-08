/**
 * Runtime entry for the third-party CSS-in-JS APIs Devup UI absorbs.
 *
 * These exist only so a rewritten import resolves — none of them belong to the
 * Devup UI API, so they stay out of `@devup-ui/react` and never widen what a
 * project using Devup UI directly has to look at. The build plugins point
 * rewritten imports here automatically.
 */

import type { ReactNode } from 'react'

import type { DevupPropsWithTheme } from '../types/props'
import type { StyledTheme } from '../utils/theme-vars'

export { Global } from '../components/Global'
export { ThemeProvider } from '../components/ThemeProvider'
export { useStyledTheme as useTheme } from '../hooks/use-styled-theme'
export { createGlobalStyle } from '../utils/create-global-style'
export {
  isStyledComponent,
  ServerStyleSheet,
  StyleSheetManager,
} from '../utils/styled-compat'
export type { StyledTheme } from '../utils/theme-vars'
export { withTheme } from '../utils/with-theme'
/**
 * Emotion's `jsx`: the build compiles the `css` props it is given, leaving
 * React's own element.
 */
export { createElement as jsx } from 'react'

/** What Emotion's `css` prop composes; the build compiles it to classes. */
export type CssInterpolation =
  | DevupPropsWithTheme
  | string
  | false
  | null
  | undefined
  | readonly CssInterpolation[]

/** Emotion's `css` prop: styles, or a function of the theme giving them. */
export type CssProp =
  CssInterpolation | ((theme: StyledTheme) => CssInterpolation)

/** A class `cx` composes, or an object giving each class its condition. */
export type ClassNamesArg =
  | string
  | false
  | null
  | undefined
  | Record<string, unknown>
  | readonly ClassNamesArg[]

/** What the child function of Emotion's `ClassNames` takes. */
export interface ClassNamesContent {
  css: (...styles: CssInterpolation[]) => string
  cx: (...classes: ClassNamesArg[]) => string
  theme: StyledTheme
}

/**
 * Emotion's `ClassNames`: the build renders what its child function gives in
 * its place, each `css` and `cx` call compiled to classes.
 */
export function ClassNames(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: { children: (content: ClassNamesContent) => ReactNode },
): ReactNode {
  throw new Error('Cannot run on the runtime')
}
