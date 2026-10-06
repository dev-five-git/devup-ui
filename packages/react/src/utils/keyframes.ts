import type { DevupCommonProps } from '../types/props'
import type { StyleTemplateValue } from './css'

type FrameProps = DevupCommonProps & {
  styleOrder?: never
  'style-order'?: never
}

export interface KeyframesProps {
  styleOrder?: never
  'style-order'?: never
  from?: FrameProps | string
  to?: FrameProps | string
  [key: `${number}%`]: FrameProps | string
}

export function keyframes(props: KeyframesProps): string
export function keyframes(props: Record<string, FrameProps>): string
export function keyframes(
  strings: TemplateStringsArray,
  ...values: StyleTemplateValue[]
): string
export function keyframes(): string

export function keyframes(
  _strings?: TemplateStringsArray | KeyframesProps,
  ..._values: StyleTemplateValue[]
): string {
  throw new Error('Cannot run on the runtime')
}
