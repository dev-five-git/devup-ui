import type { SupportedCssProperties } from './props/css-properties'
export type StyleXScalar = string | number
export type StyleXAtRule =
  `@media ${string}` | `@supports ${string}` | `@container ${string}`

/** Variable conditions may nest at-rules, but cannot depend on an element. */
export type StyleXVariableValue<T = StyleXScalar> =
  | T
  | null
  | {
      readonly default?: StyleXVariableValue<T>
      readonly [condition: StyleXAtRule]: StyleXVariableValue<T>
    }

export type StyleXValue<T = StyleXScalar> =
  | T
  | null
  | {
      readonly default?: StyleXValue<T>
      readonly [condition: StyleXAtRule | `:${string}`]: StyleXValue<T>
    }

export type StyleXCheckedValue<V, C extends string> = V extends object
  ? {
      readonly [K in keyof V]: K extends C ? StyleXCheckedValue<V[K], C> : never
    }
  : V

export type StyleXCheckedVariableValue<V> = StyleXCheckedValue<
  V,
  'default' | StyleXAtRule
>

type CssProperties = SupportedCssProperties

export type StyleXDeclarations = {
  readonly [K in keyof CssProperties]?: StyleXValue<CssProperties[K]>
} & {
  readonly [property: `--${string}`]: StyleXValue
}

/** Dynamic namespace bodies and named declaration blocks are flat. */
export type StyleXFlatDeclarations = {
  readonly [K in keyof CssProperties]?: CssProperties[K]
} & {
  readonly [property: `--${string}`]: StyleXScalar
}

export type StyleXDynamicDeclarations = {
  readonly [K in keyof CssProperties]?: CssProperties[K] | null
} & {
  readonly [property: `--${string}`]: StyleXScalar | null
}

export interface StyleXTypes {
  angle<const T extends StyleXVariableValue<string | number>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  color<const T extends StyleXVariableValue<string>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  image<const T extends StyleXVariableValue<string>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  integer<const T extends StyleXVariableValue<number>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  length<const T extends StyleXVariableValue<string | number>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  lengthPercentage<const T extends StyleXVariableValue<string | number>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  number<const T extends StyleXVariableValue<number>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  percentage<const T extends StyleXVariableValue<string | number>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  resolution<const T extends StyleXVariableValue<string>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  time<const T extends StyleXVariableValue<string>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  transformFunction<const T extends StyleXVariableValue<string>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  transformList<const T extends StyleXVariableValue<string>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
  url<const T extends StyleXVariableValue<string>>(
    value: T & StyleXCheckedVariableValue<T>,
  ): T
}
