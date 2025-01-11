import type { DevupProps } from '../types/props'
import type { Merge } from '../types/utils'

export function Box(props: Merge<React.ComponentProps<'div'>, DevupProps>) {
  return <div {...(props as React.ComponentProps<'div'>)} />
}
