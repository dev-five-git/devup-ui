import { DevupCommonProps } from '../types/props'
import type { DevupSelectorProps } from '../types/props/selector'

export function css(props: DevupCommonProps): string
export function css(strings: TemplateStringsArray): string

export function css(
  strings: TemplateStringsArray | (DevupCommonProps & DevupSelectorProps),
): string {
  if (Array.isArray(strings)) {
    return strings.join('')
  }
  return strings as string
}
