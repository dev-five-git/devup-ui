import type { ComponentType } from 'react'

import { useStyledTheme } from '../hooks/use-styled-theme'
import type { StyledTheme } from './theme-vars'

export function withTheme<P extends { theme?: StyledTheme }>(
  Component: ComponentType<P>,
): ComponentType<Omit<P, 'theme'>> {
  return function WithTheme(props: Omit<P, 'theme'>) {
    return <Component {...(props as P)} theme={useStyledTheme()} />
  }
}
