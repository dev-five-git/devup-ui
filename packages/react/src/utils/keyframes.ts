import type { DevupCommonProps } from '../types/props'

export function keyframes(
  props: Record<(string & {}) | 'from' | 'to', DevupCommonProps>,
): string
export function keyframes(strings: TemplateStringsArray): string
export function keyframes(): string

export function keyframes(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  strings?:
    | TemplateStringsArray
    | Record<(string & {}) | 'from' | 'to', DevupCommonProps>,
): string {
  throw new Error('Cannot run on the runtime')
}
