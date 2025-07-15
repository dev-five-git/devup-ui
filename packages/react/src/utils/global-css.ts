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
