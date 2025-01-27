import { DevupCommonProps } from '../types/props'
import type { DevupSelectorProps } from '../types/props/selector'

export function css(props: DevupCommonProps & DevupSelectorProps): string
export function css(strings: TemplateStringsArray): string
export function css(): string

export function css(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  strings?: TemplateStringsArray | (DevupCommonProps & DevupSelectorProps),
): string {
  throw new Error('Cannot run on the runtime')
}
