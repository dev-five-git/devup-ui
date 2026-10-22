import type { DevupPropsWithTheme } from '../types/props'

/** What a tagged `css`, `keyframes` or `globalCss` template takes between its text: values the build knows, and what `css()` and `keyframes()` give */
export type StyleTemplateValue = string | number | boolean | null | undefined

/** A part `css()` composes: a style object, a class (another `css()` result or a plain class), a falsy part the build skips, or an array of parts */
export type CssPart =
  DevupPropsWithTheme | string | false | null | undefined | readonly CssPart[]

export function css(props: DevupPropsWithTheme): string
export function css(
  strings: TemplateStringsArray,
  ...values: StyleTemplateValue[]
): string
export function css(...parts: CssPart[]): string

export function css(..._parts: unknown[]): string {
  throw new Error('Cannot run on the runtime')
}
