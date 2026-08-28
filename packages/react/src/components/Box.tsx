import type {
  DevupComponentBaseProps,
  DevupComponentProps,
} from '../types/props'
import type { Merge } from '../types/utils'

export function Box(
  props: Merge<DevupComponentBaseProps<'div'>, DevupComponentProps<'div'>>,
): React.ReactElement
export function Box<T extends React.ElementType = 'div'>(
  props: Merge<DevupComponentBaseProps<T>, DevupComponentProps<T>>,
): React.ReactElement
export function Box<T extends React.ElementType = 'div'>(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: Merge<DevupComponentBaseProps<T>, DevupComponentProps<T>>,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
