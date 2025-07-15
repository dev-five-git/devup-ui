import type { DevupCommonProps } from '../types/props'
import type {
  DevupSelectorProps,
  DevupThemeSelectorProps,
} from '../types/props/selector'

type CssValue = DevupCommonProps | DevupSelectorProps | DevupThemeSelectorProps

type GlobalCssProps = {
  [key in
    | keyof HTMLElementTagNameMap
    | keyof SVGElementTagNameMap
    | (string & {})]?: CssValue
}

export function globalCss(
  strings?:
    | TemplateStringsArray
    | (Omit<GlobalCssProps, 'imports'> & {
        imports?: string[]
      }),
): never

export function globalCss(): never

export function globalCss(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  strings?:
    | TemplateStringsArray
    | (Omit<GlobalCssProps, 'imports'> & {
        imports?: string[]
      }),
): never {
  throw new Error('Cannot run on the runtime')
}
