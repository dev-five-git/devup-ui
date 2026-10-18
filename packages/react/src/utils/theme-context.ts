'use client'

import { createContext } from 'react'

import type { StyledTheme } from './theme-vars'

/**
 * The theme object the nearest compat `ThemeProvider` gives, as
 * styled-components and Emotion hand it to `useTheme`, `withTheme` and
 * `ThemeConsumer`. Styles keep reading CSS variables; only JS reads this.
 */
export const StyledThemeContext = createContext<StyledTheme | undefined>(
  undefined,
)
