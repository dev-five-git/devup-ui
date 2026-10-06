import type { DevupPropsWithTheme } from '../types/props'
import type { StyleOrder } from '../types/props/selector'

/** Static scalar template inputs; the build proves their values and context. */
export type StyleTemplateValue = string | number | boolean | null | undefined

export type StyleObjectProps = DevupPropsWithTheme & {
  'style-order'?: StyleOrder | `${StyleOrder}`
}

/** A style object, class, skipped falsy part or readonly nested composition. */
export type CssPart =
  | StyleObjectProps
  | string
  | false
  | null
  | undefined
  | (readonly CssPart[] & { readonly raw?: never })

export function css(props: StyleObjectProps): string
export function css(
  strings: TemplateStringsArray,
  ...values: StyleTemplateValue[]
): string
export function css(...parts: CssPart[]): string

export function css(..._parts: unknown[]): string {
  throw new Error('Cannot run on the runtime')
}
