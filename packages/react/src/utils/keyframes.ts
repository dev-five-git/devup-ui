import type { DevupCommonProps } from '../types/props'

interface KeyframesProps {
  from?: DevupCommonProps | string
  to?: DevupCommonProps | string
  [key: `${number}%`]: DevupCommonProps | string
}

export function keyframes(props: KeyframesProps): string
export function keyframes(props: Record<string, DevupCommonProps>): string
export function keyframes(strings: TemplateStringsArray): string
export function keyframes(): string

export function keyframes(
  _strings?: TemplateStringsArray | KeyframesProps,
): string {
  throw new Error('Cannot run on the runtime')
}
