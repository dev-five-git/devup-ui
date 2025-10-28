import type { DevupCommonProps } from '../types/props'
import type {
  AdvancedSelector,
  CamelCase,
  DevupThemeSelectorProps,
  ExtractSelector,
  SimpleSelector,
} from '../types/props/selector'
import type { DevupSelectorProps } from '../types/props/selector'

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
    Exclude<AdvancedSelector, SimpleSelector>
  >]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps & {
      params?: string[]
    }
} & {
  [K in GlobalCssKeys<SimpleSelector> | (string & {})]?: DevupCommonProps &
    DevupSelectorProps &
    DevupThemeSelectorProps
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
export function globalCss(
  strings?:
    | TemplateStringsArray
    | (Omit<GlobalCssProps, 'imports'> & {
        imports?: Import[]
        fontFaces?: FontFaceProps[]
      }),
): void

export function globalCss(): void

export function globalCss(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  strings?:
    | TemplateStringsArray
    | (Omit<GlobalCssProps, 'imports'> & {
        imports?: Import[]
      }),
): void {
  throw new Error('Cannot run on the runtime')
}
