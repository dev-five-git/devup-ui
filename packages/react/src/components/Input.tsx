import type {
  DevupComponentBaseProps,
  DevupComponentProps,
} from '../types/props'
import type { Merge } from '../types/utils'

export function Input<T extends React.ElementType = 'input'>(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: Merge<DevupComponentBaseProps<T>, DevupComponentProps<T>>,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
