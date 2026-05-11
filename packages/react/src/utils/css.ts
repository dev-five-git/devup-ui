import type { DevupPropsWithTheme } from '../types/props'

export function css(props: DevupPropsWithTheme): string
export function css(strings: TemplateStringsArray): string
export function css(): string

export function css(
  _strings?: TemplateStringsArray | DevupPropsWithTheme,
): string {
  throw new Error('Cannot run on the runtime')
}
