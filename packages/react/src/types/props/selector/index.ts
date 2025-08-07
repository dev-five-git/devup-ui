import type { Pseudos } from 'csstype'

import type { ResponsiveValue } from '../../responsive-value'
import type { DevupTheme } from '../../theme'
import type { DevupProps } from '../index'

type CamelCase<S extends string> =
  S extends Lowercase<S>
    ? S extends `${infer F}-${infer RF}${infer R}`
      ? `${F}${Uppercase<RF>}${CamelCase<R>}`
      : S
    : CamelCase<Lowercase<S>>

type PascalCase<S extends string> = Capitalize<CamelCase<S>>

export type SelectorProps<T = DevupProps> = ResponsiveValue<T | string | false>
export type DevupThemeSelectorProps = keyof DevupTheme extends undefined
  ? Partial<Record<`_theme${string}`, SelectorProps>>
  : Partial<Record<`_theme${PascalCase<keyof DevupTheme>}`, SelectorProps>>

export type NormalSelector = Exclude<
  Pseudos,
  `:-${string}` | `::-${string}` | `${string}()`
>
export type ExtractSelector<T = NormalSelector> = T extends `::${infer R}`
  ? R
  : T extends `:${infer R}`
    ? R
    : never
export type CommonSelectorProps = {
  [K in ExtractSelector as
    | `_${CamelCase<K>}`
    | `_group${PascalCase<K>}`]?: SelectorProps
}

export type Selectors = Partial<
  Record<
    (string & {}) | `&${NormalSelector}` | `_${CamelCase<ExtractSelector>}`,
    SelectorProps
  >
>

export interface DevupSelectorProps extends CommonSelectorProps {
  // media query
  _print?: SelectorProps

  selectors?: Selectors

  styleOrder?: number
}
