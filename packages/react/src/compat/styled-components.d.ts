declare module 'styled-components' {
  import {
    type CssPart,
    type DevupProps,
    type KeyframesProps,
    styled,
    type StyleTemplateValue,
  } from '@devup-ui/react'
  import type { StyledTheme } from '@devup-ui/react/compat'
  import type { Properties } from 'csstype-extra'
  import type { ComponentType, Consumer, Context, ReactNode } from 'react'

  const ruleSet: unique symbol
  const keyframesName: unique symbol

  /**
   * What `css` gives: the build writes the class it makes of the rules in its
   * place, so the value is that class and only the build-known parts compose it.
   */
  export type RuleSet = string & { readonly [ruleSet]: true }

  /** What `keyframes` gives: the name the build gives the animation. */
  export type Keyframes = string & { readonly [keyframesName]: true }

  /** The theme `ThemeProvider` publishes as CSS variables; augment it as in styled-components. */
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type
  export interface DefaultTheme extends StyledTheme {}

  export type CSSProperties = Properties
  export type CSSObject = DevupProps
  export type CSSPseudos = Record<`:${string}`, CSSObject>
  export type CSSKeyframes = KeyframesProps
  export type CSSProp = CssPart

  export function css(props: DevupProps): RuleSet
  export function css(
    strings: TemplateStringsArray,
    ...values: StyleTemplateValue[]
  ): RuleSet
  export function css(...parts: CssPart[]): RuleSet

  export function keyframes(props: KeyframesProps): Keyframes
  export function keyframes(
    strings: TemplateStringsArray,
    ...values: StyleTemplateValue[]
  ): Keyframes

  export function useTheme(): DefaultTheme
  export function withTheme<P extends { theme?: DefaultTheme }>(
    component: ComponentType<P>,
  ): ComponentType<Omit<P, 'theme'>>
  export function ThemeProvider(props: {
    theme?: DefaultTheme
    children?: ReactNode
  }): ReactNode

  /**
   * Not compiled: the build keeps these imports on `styled-components`, which
   * must then be installed.
   */
  export const ThemeConsumer: Consumer<DefaultTheme | undefined>
  export const ThemeContext: Context<DefaultTheme | undefined>

  export {
    createGlobalStyle,
    isStyledComponent,
    ServerStyleSheet,
    StyleSheetManager,
  } from '@devup-ui/react/compat'

  export { styled }
  export default styled
}
