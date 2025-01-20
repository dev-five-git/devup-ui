import { DevupCommonProps } from '../types/props'

export function css(props: DevupCommonProps): string
export function css(strings: TemplateStringsArray): string

export function css(strings: TemplateStringsArray | DevupCommonProps): string {
  if (Array.isArray(strings)) {
    return strings.join('')
  }
  return strings as string
}
