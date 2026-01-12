import type { AdvancedPseudos, SimplePseudos } from 'csstype-extra'

import type { ResponsiveValue } from '../../responsive-value'
import type { DevupTheme } from '../../theme'
import type { DevupProps } from '../index'

export type CamelCase<S extends string> =
  S extends Lowercase<S>
    ? S extends `${infer F}-${infer RF}${infer R}`
      ? `${F}${Uppercase<RF>}${CamelCase<R>}`
      : S
    : CamelCase<Lowercase<S>>

type PascalCase<S extends string> = Capitalize<CamelCase<S>>

export type SelectorProps<T> = ResponsiveValue<T | string | false>
export type DevupThemeSelectorProps = keyof DevupTheme extends undefined
  ? Partial<Record<`_theme${string}`, SelectorProps<DevupProps>>>
  : Partial<
      Record<`_theme${PascalCase<keyof DevupTheme>}`, SelectorProps<DevupProps>>
    >

export type NormalizedSelector<T> = Exclude<T, `:-${string}` | `::-${string}`>
export type SimpleSelector = NormalizedSelector<SimplePseudos>

export type AdvancedSelector = NormalizedSelector<AdvancedPseudos>

export type ExtractSelector<T> = T extends `::${infer R}`
  ? R
  : T extends `:${infer R}`
    ? R
    : never

export type AdvancedSelectorProps = {
  [K in ExtractSelector<Exclude<AdvancedSelector, SimpleSelector>> as
    | `_${CamelCase<K>}`
    | `_group${PascalCase<K>}`]?: SimpleSelectorProps &
    AdvancedSelectorProps & {
      params: string[]
      selectors?: Selectors
    }
}

export type MultipleSelectorProps = {
  [K in ExtractSelector<Extract<AdvancedSelector, SimpleSelector>> as
    | `_${CamelCase<K>}`
    | `_group${PascalCase<K>}`]?: SelectorProps<DevupProps> & {
    params?: string[]
  }
}

export type SimpleSelectorProps = {
  [K in ExtractSelector<Exclude<SimpleSelector, AdvancedSelector>> as
    | `_${CamelCase<K>}`
    | `_group${PascalCase<K>}`]?: SelectorProps<DevupProps>
}

export type Selectors = Partial<
  Record<
    | (string & {})
    | `&${SimpleSelector}`
    | `_${CamelCase<ExtractSelector<SimpleSelector>>}`,
    SelectorProps<DevupProps>
  >
>

export type AtRuleRecord = Partial<
  Record<string & {}, SelectorProps<DevupProps>>
>

export interface DevupSelectorProps
  extends SimpleSelectorProps, AdvancedSelectorProps {
  // media query
  _print?: SelectorProps<DevupProps>
  _screen?: SelectorProps<DevupProps>
  _speech?: SelectorProps<DevupProps>
  _all?: SelectorProps<DevupProps>

  // at-rules (underscore prefix)
  _container?: AtRuleRecord
  _media?: AtRuleRecord
  _supports?: AtRuleRecord

  // at-rules (@ prefix)
  '@container'?: AtRuleRecord
  '@media'?: AtRuleRecord
  '@supports'?: AtRuleRecord

  selectors?: Selectors

  styleOrder?: number
}
