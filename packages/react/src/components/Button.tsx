import type { DevupProps } from '../types/props'
import type { Merge } from '../types/utils'

export function Button(
  props: Merge<React.ComponentProps<'button'>, DevupProps>,
) {
  return <button {...(props as React.ComponentProps<'button'>)} />
}
