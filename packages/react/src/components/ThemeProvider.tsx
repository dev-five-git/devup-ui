import type { ReactNode } from 'react'

import type { StyledTheme } from '../utils/theme-vars'
import { themeToCssVariables } from '../utils/theme-vars'

interface ThemeProviderProps {
  theme?: StyledTheme
  children?: ReactNode
}

export function ThemeProvider({ theme, children }: ThemeProviderProps) {
  return <div style={themeToCssVariables(theme)}>{children}</div>
}
