import type { DevupProps } from '../types/props'
import type { Merge } from '../types/utils'

export function Text(props: Merge<React.ComponentProps<'span'>, DevupProps>) {
  return <span {...(props as React.ComponentProps<'span'>)} />
}
