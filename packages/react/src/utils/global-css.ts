import type { DevupCommonProps } from '../types/props'
import type { DevupThemeSelectorProps } from '../types/props/selector'
import type { DevupSelectorProps } from '../types/props/selector'

type GlobalCssProps = {
  [key in
    | keyof HTMLElementTagNameMap
    | keyof SVGElementTagNameMap
    | (string & {})]?:
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

export function globalCss(
  strings?:
    | TemplateStringsArray
    | (Omit<GlobalCssProps, 'imports'> & {
        imports?: string[]
        fontFaces?: FontFaceProps[]
      }),
): void

export function globalCss(): void

export function globalCss(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  strings?:
    | TemplateStringsArray
    | (Omit<GlobalCssProps, 'imports'> & {
        imports?: string[]
      }),
): void {
  throw new Error('Cannot run on the runtime')
}
