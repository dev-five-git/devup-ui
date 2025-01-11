import type { DevupProps } from '../types/props'
import type { Merge } from '../types/utils'

export function Input(props: Merge<React.ComponentProps<'input'>, DevupProps>) {
  return <input {...(props as React.ComponentProps<'input'>)} />
}
