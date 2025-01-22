import { DevupCommonProps } from '../types/props'
import type { DevupSelectorProps } from '../types/props/selector'

export function css(props: DevupCommonProps & DevupSelectorProps): string
export function css(strings: TemplateStringsArray): string
export function css(): string

export function css(
  strings?: TemplateStringsArray | (DevupCommonProps & DevupSelectorProps),
): string {
  if (typeof strings === 'undefined') return ''
  if (Array.isArray(strings)) {
    return strings.join('')
  }
  return strings as string
}
