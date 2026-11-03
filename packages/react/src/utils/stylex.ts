import type { DevupCommonProps } from '../types/props'
import type {
  StyleXAttrs,
  StyleXCheckedNamespaces,
  StyleXCheckedVariables,
  StyleXCompiledNamespaces,
  StyleXCompiledStyle,
  StyleXConstants,
  StyleXInclude,
  StyleXNamespace,
  StyleXProps,
  StyleXStyleInput,
  StyleXThemeOverrides,
  StyleXVarGroup,
} from '../types/stylex'
import type {
  StyleXFlatDeclarations,
  StyleXScalar,
  StyleXTypes,
  StyleXVariableValue,
} from '../types/stylex-values'

export type {
  StyleXAttrs,
  StyleXCompiledStyle,
  StyleXInclude,
  StyleXInlineStyle,
  StyleXProps,
  StyleXRawStyles,
  StyleXStyleInput,
  StyleXStyles,
  StyleXThemeOverrides,
  StyleXVarGroup,
} from '../types/stylex'
export type {
  StyleXDeclarations,
  StyleXDynamicDeclarations,
  StyleXFlatDeclarations,
  StyleXTypes,
  StyleXValue,
  StyleXVariableValue,
} from '../types/stylex-values'

export function create<
  const S extends Readonly<Record<string, StyleXNamespace>>,
>(_styles: S & StyleXCheckedNamespaces<S>): StyleXCompiledNamespaces<S> {
  throw new Error('Cannot run on the runtime')
}

export function props(..._styles: readonly StyleXStyleInput[]): StyleXProps {
  throw new Error('Cannot run on the runtime')
}

export function attrs(..._styles: readonly StyleXStyleInput[]): StyleXAttrs {
  throw new Error('Cannot run on the runtime')
}

export function keyframes(
  _frames: Readonly<Record<string, DevupCommonProps>>,
): string {
  throw new Error('Cannot run on the runtime')
}

export function firstThatWorks<const V extends readonly StyleXScalar[]>(
  ..._values: V
): V[number] {
  throw new Error('Cannot run on the runtime')
}

export function include<P>(_style: StyleXCompiledStyle<P>): StyleXInclude<P> {
  throw new Error('Cannot run on the runtime')
}

export function defineVars<
  const V extends Readonly<Record<string, StyleXVariableValue>>,
>(
  _vars: V & StyleXCheckedVariables<V>,
): StyleXVarGroup<Extract<keyof V, string>> {
  throw new Error('Cannot run on the runtime')
}

export function createTheme<
  K extends string,
  const O extends Readonly<Record<string, StyleXVariableValue>>,
>(
  _vars: StyleXVarGroup<K>,
  _overrides: O &
    StyleXCheckedVariables<O> &
    StyleXThemeOverrides<NoInfer<K>> &
    Record<Exclude<keyof O, NoInfer<K>>, never>,
): StyleXCompiledStyle {
  throw new Error('Cannot run on the runtime')
}

export function createThemeContract<
  const V extends Readonly<Record<string, string | null>>,
>(_vars: V): StyleXVarGroup<Extract<keyof V, string>> {
  throw new Error('Cannot run on the runtime')
}

export function defineConsts<
  const V extends Readonly<Record<string, string | number | boolean>>,
>(_consts: V): StyleXConstants<V> {
  throw new Error('Cannot run on the runtime')
}

export function positionTry<const D extends StyleXFlatDeclarations>(
  _fallback: D & Record<Exclude<keyof D, keyof StyleXFlatDeclarations>, never>,
): string {
  throw new Error('Cannot run on the runtime')
}

export function viewTransitionClass<const D extends StyleXFlatDeclarations>(
  _styles: D & Record<Exclude<keyof D, keyof StyleXFlatDeclarations>, never>,
): string {
  throw new Error('Cannot run on the runtime')
}

export const types: StyleXTypes = new Proxy({} as StyleXTypes, {
  get() {
    return () => {
      throw new Error('Cannot run on the runtime')
    }
  },
})
