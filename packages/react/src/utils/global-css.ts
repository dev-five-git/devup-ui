import type { DevupCommonProps } from '../types/props'
import type {
  DevupThemeSelectorProps,
  ExtractSelector,
  SimpleSelector,
} from '../types/props/selector'
import type { DevupSelectorProps } from '../types/props/selector'

type GlobalCssKeys =
  | `*${SimpleSelector | ''}`
  | `${keyof HTMLElementTagNameMap}${SimpleSelector | ''}`
  | `${keyof SVGElementTagNameMap}${SimpleSelector | ''}`
  | `_${ExtractSelector<SimpleSelector>}`
  | (string & {})

type GlobalCssProps = {
  [K in GlobalCssKeys]?:
    | DevupCommonProps
    | DevupSelectorProps
    | DevupThemeSelectorProps
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
