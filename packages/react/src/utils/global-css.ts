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
  StyleOrder,
} from '../types/props/selector'
import type { StyleObjectProps, StyleTemplateValue } from './css'

type GlobalOrderProps = {
  styleOrder?: StyleOrder | `${StyleOrder}`
  'style-order'?: StyleOrder | `${StyleOrder}`
}

type GlobalCssKeys<T extends string> =
  | `*${T}`
  | `${keyof HTMLElementTagNameMap}${T}`
  | `${keyof SVGElementTagNameMap}${T}`
  | `_${CamelCase<ExtractSelector<T>>}`

export type GlobalCssProps = {
  [K in GlobalCssKeys<AdvancedSelector>]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps &
    GlobalOrderProps & {
      params: string[]
    }
} & {
  [
    K in GlobalCssKeys<Extract<AdvancedSelector, SimpleSelector>>
  ]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps &
    GlobalOrderProps & {
      params?: string[]
    }
} & {
  [K in GlobalCssKeys<SimpleSelector>]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps &
    GlobalOrderProps
} & {
  [
    K in
      | `${Exclude<keyof HTMLElementTagNameMap | keyof SVGElementTagNameMap, 's' | 'style'> | '.' | '*' | '#' | ':' | '['}${string}`
      | 's'
      | 'style'
      | `${'s' | 'style'}${' ' | ':' | '.' | '#' | '[' | '>' | '+' | '~' | ','}${string}`
  ]?: StyleObjectProps
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
} & GlobalOrderProps

interface FontFaceProps {
  styleOrder?: never
  'style-order'?: never
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
  styleOrder?: StyleOrder | `${StyleOrder}`
  'style-order'?: StyleOrder | `${StyleOrder}`
  imports?: Import[]
  fontFaces?: FontFaceProps[]
}

export function globalCss(
  strings: AdditionalGlobalCssProps | GlobalCssProps,
): void

export function globalCss(strings: Record<string, StyleObjectProps>): void

export function globalCss(
  strings?: TemplateStringsArray,
  ...values: StyleTemplateValue[]
): void

export function globalCss(): void

export function globalCss(
  _strings?:
    | TemplateStringsArray
    | (GlobalCssProps | AdditionalGlobalCssProps)
    | Record<string, StyleObjectProps>,
  ..._values: StyleTemplateValue[]
): void {
  throw new Error('Cannot run on the runtime')
}
