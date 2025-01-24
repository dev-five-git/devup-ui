import type { DevupProps } from '../types/props'
import type { Merge } from '../types/utils'

export function Text<T extends React.ElementType = 'span'>(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: Merge<React.ComponentProps<T>, DevupProps>,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
