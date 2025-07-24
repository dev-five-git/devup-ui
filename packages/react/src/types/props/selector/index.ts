import type { Pseudos } from 'csstype'

import type { ResponsiveValue } from '../../responsive-value'
import type { DevupTheme } from '../../theme'
import type { DevupCommonProps } from '../index'

type CamelCase<S extends string> =
  S extends Lowercase<S>
    ? S extends `${infer F}-${infer RF}${infer R}`
      ? `${F}${Uppercase<RF>}${CamelCase<R>}`
      : S
    : CamelCase<Lowercase<S>>

type PascalCase<S extends string> = Capitalize<CamelCase<S>>

export type DevupThemeSelectorProps = keyof DevupTheme extends undefined
  ? Record<`_theme${string}`, DevupCommonProps & DevupSelectorProps>
  : {
      [K in keyof DevupTheme as `_theme${PascalCase<K>}`]?: DevupCommonProps &
        DevupSelectorProps
    }

type SelectorProps = ResponsiveValue<
  | (DevupCommonProps & DevupSelectorProps & DevupThemeSelectorProps)
  | string
  | false
>
type NormalSelector = Exclude<
  Pseudos,
  `:-${string}` | `::-${string}` | `${string}()`
>
type ExtractSelector<T> = T extends `::${infer R}`
  ? R
  : T extends `:${infer R}`
    ? R
    : never
export type CommonSelectorProps = {
  [K in ExtractSelector<NormalSelector> as
    | `_${CamelCase<K>}`
    | `_group${PascalCase<K>}`]?: SelectorProps
}

export interface DevupSelectorProps extends CommonSelectorProps {
  // media query
  _print?: SelectorProps

  selectors?: Record<string, SelectorProps>

  styleOrder?: number
}
