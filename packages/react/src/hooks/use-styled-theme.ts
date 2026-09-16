import type { StyledTheme } from '../utils/theme-vars'
import { createThemeAccessor } from '../utils/theme-vars'

const themeAccessor = createThemeAccessor()

/**
 * styled-components compatible `useTheme`.
 *
 * Distinct from devup-ui's own `useTheme`, which reports the active theme name.
 * This one hands back CSS-variable references, matching what `ThemeProvider`
 * declares and what the extractor inlines into styles.
 */
export function useStyledTheme<T extends StyledTheme = StyledTheme>(): T {
  return themeAccessor as T
}
