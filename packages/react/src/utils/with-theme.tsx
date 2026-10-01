'use client'

import type { ComponentType, ReactNode, Ref } from 'react'
import { forwardRef, useContext } from 'react'

import { StyledThemeContext } from './theme-context'
import type { StyledTheme } from './theme-vars'

/**
 * Hands `Component` the theme the nearest `ThemeProvider` gives, unless the
 * caller passes its own `theme`, forwarding refs as styled-components does
 */
export function withTheme<P extends { theme?: StyledTheme }>(
  Component: ComponentType<P>,
) {
  const WithTheme = forwardRef(function WithTheme(
    props: Omit<P, 'theme'> & { theme?: StyledTheme },
    ref: Ref<unknown>,
  ) {
    const context = useContext(StyledThemeContext)
    return (
      <Component
        {...(props as P)}
        ref={ref}
        theme={props.theme ?? context ?? {}}
      />
    )
  })
  WithTheme.displayName = `WithTheme(${Component.displayName || Component.name || 'Component'})`
  return WithTheme
}

/**
 * styled-components' `ThemeConsumer`: renders what `children` makes of the
 * theme the nearest `ThemeProvider` gives
 */
export function ThemeConsumer({
  children,
}: {
  children: (theme: StyledTheme) => ReactNode
}): ReactNode {
  return children(useContext(StyledThemeContext) ?? {})
}
