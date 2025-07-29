import type { DevupProps } from '../types/props'

export function css(props: DevupProps): string
export function css(strings: TemplateStringsArray): string
export function css(): string

export function css(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  strings?: TemplateStringsArray | DevupProps,
): string {
  throw new Error('Cannot run on the runtime')
}
