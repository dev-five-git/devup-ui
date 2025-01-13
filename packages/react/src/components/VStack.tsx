import type { DevupProps } from '../types/props'
import type { Merge } from '../types/utils'

export function VStack(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: Merge<React.ComponentProps<'div'>, DevupProps>,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
