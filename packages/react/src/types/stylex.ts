import type {
  StyleXAtRule,
  StyleXCheckedValue,
  StyleXCheckedVariableValue,
  StyleXDeclarations,
  StyleXDynamicDeclarations,
  StyleXVariableValue,
} from './stylex-values'

declare const compiledStyle: unique symbol
declare const variableGroup: unique symbol
declare const includedStyle: unique symbol

/** Opaque build output, not a raw CSS object or a user-provided class string. */
export type StyleXCompiledStyle<P = unknown> = {
  readonly [compiledStyle]: P
}

export type StyleXStyles<P = unknown> =
  StyleXCompiledStyle<P> | false | null | undefined | readonly StyleXStyles<P>[]

export type StyleXStyleInput<P = unknown> =
  StyleXStyles<P> | string | readonly StyleXStyleInput<P>[]

export type StyleXInclude<P = unknown> = {
  readonly [includedStyle]: P
}

export type StyleXRawStyles = StyleXDeclarations & {
  readonly [pseudo: `:${string}`]: StyleXDeclarations
} & Partial<StyleXInclude>

export type StyleXNamespace =
  StyleXRawStyles | null | ((...args: never[]) => StyleXDynamicDeclarations)

type StyleLeaves<T> = T extends object ? StyleLeaves<T[keyof T]> : T
type DeclaredStyleShape<S> = {
  readonly [
    K in keyof S as K extends keyof StyleXDeclarations ? K : never
  ]: StyleLeaves<S[K]>
}

type StyleShape<S> =
  S extends StyleXInclude<infer P>
    ? Omit<P, keyof DeclaredStyleShape<S>> & DeclaredStyleShape<S>
    : DeclaredStyleShape<S>

type CheckedDeclarations<S> = {
  readonly [K in keyof S]: K extends keyof StyleXDeclarations
    ? StyleXCheckedValue<S[K], 'default' | StyleXAtRule | `:${string}`>
    : never
}

type CheckedRawStyles<S> = {
  readonly [K in keyof S]: K extends keyof StyleXDeclarations
    ? StyleXCheckedValue<S[K], 'default' | StyleXAtRule | `:${string}`>
    : K extends `:${string}`
      ? CheckedDeclarations<S[K]>
      : K extends typeof includedStyle
        ? S[K]
        : never
}

export type StyleXCompiledNamespaces<S> = {
  readonly [K in keyof S]: S[K] extends (...args: infer A) => infer R
    ? (...args: A) => StyleXCompiledStyle<StyleShape<R>>
    : StyleXCompiledStyle<StyleShape<S[K]>>
}

export type StyleXCheckedNamespaces<S> = {
  readonly [K in keyof S]: S[K] extends null
    ? null
    : S[K] extends (...args: infer A) => infer R
      ? (
          ...args: A
        ) => R &
          Record<Exclude<keyof R, keyof StyleXDynamicDeclarations>, never>
      : S[K] & CheckedRawStyles<S[K]>
}

export type StyleXVarGroup<K extends string = string> = {
  readonly [variableGroup]: K
} & Readonly<Record<K, string>>

export type StyleXThemeOverrides<K extends string> = Readonly<
  Partial<Record<K, StyleXVariableValue>>
>

export type StyleXCheckedVariables<V> = {
  readonly [K in keyof V]: StyleXCheckedVariableValue<V[K]>
}

export type StyleXConstants<C> = {
  readonly [K in keyof C]: C[K] extends string | number | boolean
    ? `${C[K]}`
    : never
}

export type StyleXInlineStyle = Readonly<
  Record<string, string | number | null | undefined>
>

export type StyleXProps = {
  readonly className: string
  readonly style?: StyleXInlineStyle
}

/** This extractor returns an inline object, unlike upstream attrs(). */
export type StyleXAttrs = {
  readonly class: string
  readonly style?: StyleXInlineStyle
}
