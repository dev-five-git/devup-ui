/// <reference types="@devup-ui/react/compat/css-prop" />

declare module '@emotion/styled' {
  import { styled } from '@devup-ui/react'

  export default styled
}

declare module '@emotion/react' {
  import type {
    CssPart,
    KeyframesProps,
    StyleTemplateValue,
  } from '@devup-ui/react'
  import type {
    ClassNamesArg,
    ClassNamesContent,
    StyledTheme,
  } from '@devup-ui/react/compat'
  import type { ComponentType, ReactNode } from 'react'

  const serializedStyles: unique symbol
  const keyframesName: unique symbol

  /**
   * What `css` gives: the build writes the class it makes of the styles in its
   * place, so the value is that class, not an object with `name` and `styles`.
   */
  export type SerializedStyles = string & {
    readonly [serializedStyles]: true
  }

  /** What `keyframes` gives: the name the build gives the animation. */
  export type Keyframes = string & { readonly [keyframesName]: true }

  /** The theme `ThemeProvider` publishes as CSS variables; augment it as in Emotion. */
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type
  export interface Theme extends StyledTheme {}

  export type { ClassNamesArg, ClassNamesContent }

  export function css(
    strings: TemplateStringsArray,
    ...values: StyleTemplateValue[]
  ): SerializedStyles
  export function css(...parts: CssPart[]): SerializedStyles

  export function keyframes(props: KeyframesProps): Keyframes
  export function keyframes(
    strings: TemplateStringsArray,
    ...values: StyleTemplateValue[]
  ): Keyframes

  export function useTheme(): Theme
  export function withTheme<P extends { theme?: Theme }>(
    component: ComponentType<P>,
  ): ComponentType<Omit<P, 'theme'>>
  export function ThemeProvider(props: {
    theme?: Theme
    children?: ReactNode
  }): ReactNode

  export {
    ClassNames,
    jsx as createElement,
    Global,
    jsx,
  } from '@devup-ui/react/compat'
}
