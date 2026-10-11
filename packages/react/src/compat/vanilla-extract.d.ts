declare module '@vanilla-extract/css' {
  type VanillaRule = Record<string, unknown>
  export type CSSVarFunction = `var(--${string})`
  export type NullableTokens = {
    readonly [key: string]: string | null | NullableTokens
  }
  export type Tokens = { readonly [key: string]: string | Tokens }
  export type Contract = {
    readonly [key: string]: CSSVarFunction | null | Contract
  }
  export type MapLeafNodes<T, Leaf> = {
    [Key in keyof T]: T[Key] extends string | null
      ? Leaf
      : T[Key] extends NullableTokens | Contract
        ? MapLeafNodes<T[Key], Leaf>
        : never
  }
  export type ThemeVars<T extends NullableTokens> = MapLeafNodes<
    T,
    CSSVarFunction
  >
  type WithOptionalLayer<T> = T & { readonly '@layer'?: string }
  type InferredVars<T extends Tokens> = Omit<ThemeVars<T>, '@layer'>

  type NativeProperties = import('csstype-extra').Properties
  export type CSSProperties = {
    [Key in keyof NativeProperties]:
      | NativeProperties[Key]
      | CSSVarFunction
      | readonly (NativeProperties[Key] | CSSVarFunction)[]
  }
  export type CSSPropertiesWithVars = CSSProperties & {
    readonly vars?: Readonly<Record<string, string>>
  }
  type Queries<Rule> = {
    readonly [
      Key in '@media' | '@supports' | '@container' | '@layer' | '@scope'
    ]?: Readonly<Record<string, Rule>>
  }
  export type GlobalStyleRule = CSSPropertiesWithVars & GlobalQueries
  interface GlobalQueries extends Queries<GlobalStyleRule> {
    readonly '@starting-style'?: GlobalStyleRule
  }
  export type StyleRule = CSSPropertiesWithVars & {
    readonly [
      Key in import('csstype-extra').SimplePseudos
    ]?: CSSPropertiesWithVars
  } & {
    readonly selectors?: Readonly<Record<string, GlobalStyleRule>>
  } & StyleQueries
  interface StyleQueries extends Queries<StyleRule> {
    readonly '@starting-style'?: StyleRule
  }
  export type ClassNames = string | readonly ClassNames[]
  export type ComplexStyleRule = StyleRule | readonly (StyleRule | ClassNames)[]
  export type CSSKeyframes = Readonly<Record<string, CSSPropertiesWithVars>>
  type FontDescriptors = Pick<
    NativeProperties,
    | 'fontStyle'
    | 'fontWeight'
    | 'fontStretch'
    | 'fontVariant'
    | 'fontFeatureSettings'
    | 'fontVariationSettings'
  >
  export type FontFaceRule = {
    [Key in keyof FontDescriptors]:
      FontDescriptors[Key] | readonly FontDescriptors[Key][]
  } & {
    readonly src: string | readonly string[]
    readonly fontDisplay?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
    readonly unicodeRange?: string | readonly string[]
    readonly sizeAdjust?: string
    readonly ascentOverride?: string
    readonly descentOverride?: string
    readonly lineGapOverride?: string
  }
  export type PropertySyntax =
    | '*'
    | '<angle>'
    | '<color>'
    | '<custom-ident>'
    | '<image>'
    | '<integer>'
    | '<length-percentage>'
    | '<length>'
    | '<number>'
    | '<percentage>'
    | '<resolution>'
    | '<string>'
    | '<time>'
    | '<transform-function>'
    | '<transform-list>'
    | '<url>'
    | (string & {})
  export type VarDeclaration =
    | {
        readonly syntax: '*'
        readonly inherits: boolean
        readonly initialValue?: string
      }
    | {
        readonly syntax:
          Exclude<PropertySyntax, '*'> | readonly PropertySyntax[]
        readonly inherits: boolean
        readonly initialValue: string
      }
  type LayerOptions = { readonly parent?: string }

  export function style(rule: ComplexStyleRule, debugId?: string): string
  export function globalStyle(selector: string, rule: GlobalStyleRule): void
  export function styleVariants<
    T extends Record<string | number, ComplexStyleRule>,
  >(variants: T, debugId?: string): Record<keyof T, string>
  export function styleVariants<Data extends Record<string | number, unknown>>(
    data: Data,
    mapData: (value: Data[keyof Data], key: keyof Data) => ComplexStyleRule,
    debugId?: string,
  ): Record<keyof Data, string>
  export function keyframes(frames: CSSKeyframes, debugId?: string): string
  export function fontFace(
    rule: FontFaceRule | readonly FontFaceRule[],
    debugId?: string,
  ): string
  export function globalFontFace(
    name: string,
    rule: FontFaceRule | readonly FontFaceRule[],
  ): void
  export function createVar(debugId?: string): CSSVarFunction
  export function createVar(
    declaration: VarDeclaration,
    debugId?: string,
  ): CSSVarFunction
  export function fallbackVar(...values: [string, ...string[]]): CSSVarFunction
  export function createContainer(debugId?: string): string
  export function layer(options: LayerOptions, debugId?: string): string
  export function layer(debugId?: string): string
  export function globalLayer(options: LayerOptions, name: string): string
  export function globalLayer(name: string): string
  export function createThemeContract<T extends NullableTokens>(
    shape: T,
  ): ThemeVars<T>
  export function createGlobalThemeContract<T extends Tokens>(
    names: T,
  ): ThemeVars<T>
  export function createGlobalThemeContract<T extends NullableTokens>(
    shape: T,
    mapFn: (value: string | null, path: string[]) => string,
  ): ThemeVars<T>
  export function assignVars<T extends Contract>(
    contract: T,
    values: MapLeafNodes<T, string>,
  ): Record<CSSVarFunction, string>
  export function createTheme<T extends Contract>(
    contract: T,
    values: WithOptionalLayer<MapLeafNodes<T, string>>,
    debugId?: string,
  ): string
  export function createTheme<T extends Tokens>(
    values: WithOptionalLayer<T>,
    debugId?: string,
  ): [className: string, vars: InferredVars<T>]
  export function createGlobalTheme<T extends Tokens>(
    selector: string,
    values: WithOptionalLayer<T>,
  ): InferredVars<T>
  export function createGlobalTheme<T extends Contract>(
    selector: string,
    contract: T,
    values: WithOptionalLayer<MapLeafNodes<T, string>>,
  ): void

  // Existing declarations for APIs outside the evaluator registration.
  export function globalKeyframes(name: string, frames: VanillaRule): void
  export function composeStyles(...classNames: string[]): string
  export function generateIdentifier(debugId?: string): string
}
