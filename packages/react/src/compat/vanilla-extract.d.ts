declare module '@vanilla-extract/css' {
  export { keyframes, css as style } from '@devup-ui/react'

  type VanillaRule = Record<string, unknown>

  /**
   * Keeps vanilla-extract's own two-argument shape. The extractor folds the
   * selector back into the object `globalCss` takes, so the call compiles even
   * though the devup-ui signature it maps onto is single-argument.
   */
  export function globalStyle(selector: string, rule: VanillaRule): void

  /**
   * Declared but not rewritten: these only run inside `.css.ts` / `.css.js`
   * stylesheets, where the extractor evaluates the module and replaces every
   * call with its generated output.
   */
  export function styleVariants<T extends Record<string, VanillaRule>>(
    variants: T,
  ): Record<keyof T, string>
  export function createVar(): string
  export function fallbackVar(...values: string[]): string
  export function fontFace(rule: VanillaRule): string
  export function globalFontFace(name: string, rule: VanillaRule): void
  export function globalKeyframes(name: string, frames: VanillaRule): void
  export function createTheme<T>(contract: T, values: unknown): string
  export function createTheme<T extends VanillaRule>(values: T): [string, T]
  export function createThemeContract<T>(shape: T): T
  export function assignVars(
    contract: unknown,
    values: unknown,
  ): Record<string, string>
  export function composeStyles(...classNames: string[]): string
  export function layer(options?: unknown): string
  export function globalLayer(options: unknown, name?: string): string
  export function createContainer(): string
  export function generateIdentifier(debugId?: string): string
}
