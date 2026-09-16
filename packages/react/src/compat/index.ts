/**
 * Runtime entry for the third-party CSS-in-JS APIs Devup UI absorbs.
 *
 * These exist only so a rewritten import resolves — none of them belong to the
 * Devup UI API, so they stay out of `@devup-ui/react` and never widen what a
 * project using Devup UI directly has to look at. The build plugins point
 * rewritten imports here automatically.
 */

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
