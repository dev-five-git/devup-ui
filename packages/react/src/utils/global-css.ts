import type { DevupCommonProps } from '../types/props'
import type {
  AdvancedSelector,
  AtRuleKey,
  CamelCase,
  DevupSelectorProps,
  DevupThemeSelectorProps,
  ExtractSelector,
  MediaShorthand,
  SimpleSelector,
} from '../types/props/selector'

type GlobalCssKeys<T extends string> =
  | `*${T}`
  | `${keyof HTMLElementTagNameMap}${T}`
  | `${keyof SVGElementTagNameMap}${T}`
  | `_${CamelCase<ExtractSelector<T>>}`

export type GlobalCssProps = {
  [K in GlobalCssKeys<AdvancedSelector>]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps & {
      params: string[]
    }
} & {
  [
    K in GlobalCssKeys<Extract<AdvancedSelector, SimpleSelector>>
  ]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps & {
      params?: string[]
    }
} & {
  [K in GlobalCssKeys<SimpleSelector>]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps
} & {
  [
    K in `${keyof HTMLElementTagNameMap | keyof SVGElementTagNameMap | '.' | '*' | '#' | ':' | '['}${string}`
  ]?: DevupCommonProps & DevupSelectorProps & DevupThemeSelectorProps
} & {
  // Top-level at-rules wrap whole selector maps: `'@media print': { body: … }`.
  [K in AtRuleKey | MediaShorthand]?: GlobalCssProps
} & {
  [
    K in
      | '_media'
      | '_supports'
      | '_container'
      | '@media'
      | '@supports'
      | '@container'
  ]?: Record<string, GlobalCssProps>
}

interface FontFaceProps {
  fontFamily: string
  src: string
  fontWeight?: string | number
  fontStyle?: string
  fontDisplay?: string
  unicodeRange?: string
  fontVariant?: string
  ascentOverride?: string
  descentOverride?: string
  fontStretch?: string
  lineGapOverride?: string
  sizeAdjust?: string
  fontFeatureSettings?: string
  fontVariationSettings?: string
}

type Import = { url: string; query?: string } | string
export interface AdditionalGlobalCssProps {
  imports?: Import[]
  fontFaces?: FontFaceProps[]
}

export function globalCss(
  strings: AdditionalGlobalCssProps | GlobalCssProps,
): void

export function globalCss(
  strings: Record<
    string,
    DevupCommonProps & DevupSelectorProps & DevupThemeSelectorProps
  >,
): void

export function globalCss(strings?: TemplateStringsArray): void

export function globalCss(): void

export function globalCss(
  _strings?:
    | TemplateStringsArray
    | (GlobalCssProps | AdditionalGlobalCssProps)
    | Record<
        string,
        DevupCommonProps & DevupSelectorProps & DevupThemeSelectorProps
      >,
): void {
  throw new Error('Cannot run on the runtime')
}
