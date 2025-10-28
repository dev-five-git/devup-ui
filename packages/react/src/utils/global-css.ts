import type { DevupCommonProps } from '../types/props'
import type {
  AdvancedSelector,
  CamelCase,
  DevupSelectorProps,
  DevupThemeSelectorProps,
  ExtractSelector,
  SimpleSelector,
} from '../types/props/selector'

type GlobalCssKeys<T extends string> =
  | `*${T}`
  | `${keyof HTMLElementTagNameMap}${T}`
  | `${keyof SVGElementTagNameMap}${T}`
  | `_${CamelCase<ExtractSelector<T>>}`

type GlobalCssProps = {
  [K in GlobalCssKeys<AdvancedSelector>]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps & {
      params: string[]
    }
} & {
  [K in GlobalCssKeys<
    Extract<AdvancedSelector, SimpleSelector>
  >]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps & {
      params?: string[]
    }
} & {
  [K in GlobalCssKeys<SimpleSelector>]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps
} & {
  [K in
    | keyof HTMLElementTagNameMap
    | keyof SVGElementTagNameMap
    | '*']?: DevupCommonProps & DevupSelectorProps & DevupThemeSelectorProps
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
interface AdditionalGlobalCssProps {
  imports?: Import[]
  fontFaces?: FontFaceProps[]
}

export function globalCss(
  strings: AdditionalGlobalCssProps & GlobalCssProps,
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
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  strings?:
    | TemplateStringsArray
    | (GlobalCssProps & AdditionalGlobalCssProps)
    | Record<
        string,
        DevupCommonProps & DevupSelectorProps & DevupThemeSelectorProps
      >,
): void {
  throw new Error('Cannot run on the runtime')
}
