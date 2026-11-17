'use client'

import type { ComponentType, PropsWithoutRef, ReactNode, Ref } from 'react'
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
    props: PropsWithoutRef<Omit<P, 'theme'> & { theme?: StyledTheme }>,
    ref: Ref<unknown>,
  ) {
    const context = useContext(StyledThemeContext)
    const own = (props as unknown as { theme?: StyledTheme }).theme
    return (
      <Component
        {...(props as unknown as P)}
        ref={ref}
        theme={own ?? context ?? {}}
      />
    )
  })
  WithTheme.displayName = `WithTheme(${Component.displayName || Component.name || 'Component'})`
  return hoistStatics(WithTheme, Component)
}

/**
 * Statics React or a function defines itself, which the wrapper must keep its
 * own, as `hoist-non-react-statics` skips them
 */
const KNOWN_STATICS = new Set<PropertyKey>([
  '$$typeof',
  'arguments',
  'arity',
  'caller',
  'childContextTypes',
  'compare',
  'contextType',
  'contextTypes',
  'defaultProps',
  'displayName',
  'getDefaultProps',
  'getDerivedStateFromError',
  'getDerivedStateFromProps',
  'length',
  'mixins',
  'name',
  'propTypes',
  'prototype',
  'render',
  'type',
])

/**
 * Copies the statics `source` carries that are not React's onto `target`,
 * as styled-components' `withTheme` does, so `Themed.Item` keeps working
 */
export function hoistStatics<T extends object>(target: T, source: object): T {
  for (const key of Reflect.ownKeys(source)) {
    if (
      KNOWN_STATICS.has(key) ||
      Object.prototype.hasOwnProperty.call(target, key)
    ) {
      continue
    }
    Object.defineProperty(
      target,
      key,
      Object.getOwnPropertyDescriptor(source, key) as PropertyDescriptor,
    )
  }
  return target
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
