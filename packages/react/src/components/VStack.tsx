import type { DevupComponentProps } from '../types/props'
import type { Merge } from '../types/utils'

export function VStack<T extends React.ElementType = 'div'>(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: Merge<React.ComponentProps<T>, DevupComponentProps<T>>,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
