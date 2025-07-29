import type { DevupPropsWithTheme } from '../types/props'

export function css(props: DevupPropsWithTheme): string
export function css(strings: TemplateStringsArray): string
export function css(): string

export function css(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  strings?: TemplateStringsArray | DevupPropsWithTheme,
): string {
  throw new Error('Cannot run on the runtime')
}
