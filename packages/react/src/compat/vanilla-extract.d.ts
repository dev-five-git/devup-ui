declare module '@vanilla-extract/css' {
  export { keyframes, css as style } from '@devup-ui/react'

  type VanillaRule = Record<string, unknown>

  /** A rule, or rules composed in order: later declarations replace earlier ones */
  type VanillaStyle = VanillaRule | string | readonly VanillaStyle[]

  /**
   * Keeps vanilla-extract's own two-argument shape. The extractor folds the
   * selector back into the object `globalCss` takes, so the call compiles even
   * though the devup-ui signature it maps onto is single-argument.
   */
  export function globalStyle(selector: string, rule: VanillaRule): void

  /**
   * Declared but not rewritten: these only run inside `.css.ts` / `.css.js`
   * stylesheets, where the extractor evaluates the module and replaces every
   * call with its generated output. Each is one of the APIs the evaluator
   * registers.
   */
  export function styleVariants<T extends Record<string, VanillaStyle>>(
    variants: T,
  ): Record<keyof T, string>
  export function styleVariants<
    T extends Record<string, unknown>,
    Key extends keyof T = keyof T,
  >(
    data: T,
    mapData: (value: T[Key], key: Key) => VanillaStyle,
  ): Record<keyof T, string>
  export function createVar(): string
  export function fallbackVar(...values: string[]): string
  export function fontFace(rule: VanillaRule): string
  export function globalFontFace(name: string, rule: VanillaRule): void
  export function createTheme<T>(contract: T, values: unknown): string
  export function createTheme<T extends VanillaRule>(values: T): [string, T]
  export function createGlobalTheme<T extends VanillaRule>(
    selector: string,
    values: T,
  ): T
  export function createGlobalTheme<T>(
    selector: string,
    contract: T,
    values: unknown,
  ): void
  export function createThemeContract<T>(shape: T): T
  export function createGlobalThemeContract<T extends VanillaRule>(
    tokens: T,
    mapFn?: (value: unknown, path: string[]) => string,
  ): T
  export function assignVars(
    contract: unknown,
    values: unknown,
  ): Record<string, string>
  export function layer(options?: unknown): string
  export function globalLayer(options: unknown, name?: string): string
  export function createContainer(): string
}
