declare module '@stylexjs/stylex' {
  import { stylex } from '@devup-ui/react'

  export const attrs: typeof stylex.attrs
  export const create: typeof stylex.create
  export const createTheme: typeof stylex.createTheme
  export const createThemeContract: typeof stylex.createThemeContract
  export const defineConsts: typeof stylex.defineConsts
  export const defineVars: typeof stylex.defineVars
  export const firstThatWorks: typeof stylex.firstThatWorks
  export const include: typeof stylex.include
  export const keyframes: typeof stylex.keyframes
  export const positionTry: typeof stylex.positionTry
  export const props: typeof stylex.props
  export const types: typeof stylex.types
  export const viewTransitionClass: typeof stylex.viewTransitionClass

  export type StyleXAttrs = stylex.StyleXAttrs
  export type StyleXCompiledStyle<P = unknown> = stylex.StyleXCompiledStyle<P>
  export type StyleXDeclarations = stylex.StyleXDeclarations
  export type StyleXDynamicDeclarations = stylex.StyleXDynamicDeclarations
  export type StyleXFlatDeclarations = stylex.StyleXFlatDeclarations
  export type StyleXInclude<P = unknown> = stylex.StyleXInclude<P>
  export type StyleXInlineStyle = stylex.StyleXInlineStyle
  export type StyleXProps = stylex.StyleXProps
  export type StyleXRawStyles = stylex.StyleXRawStyles
  export type StyleXStyleInput<P = unknown> = stylex.StyleXStyleInput<P>
  export type StyleXStyles<P = unknown> = stylex.StyleXStyles<P>
  export type StyleXThemeOverrides<K extends string> =
    stylex.StyleXThemeOverrides<K>
  export type StyleXTypes = stylex.StyleXTypes
  export type StyleXValue<T = string | number> = stylex.StyleXValue<T>
  export type StyleXVariableValue<T = string | number> =
    stylex.StyleXVariableValue<T>
  export type StyleXVarGroup<K extends string = string> =
    stylex.StyleXVarGroup<K>

  export default stylex
}
