import type {
  DevupComponentBaseProps,
  DevupComponentProps,
} from '../types/props'
import type { Merge } from '../types/utils'

export function Text(
  props: Merge<DevupComponentBaseProps<'span'>, DevupComponentProps<'span'>>,
): React.ReactElement
export function Text<T extends React.ElementType = 'span'>(
  props: Merge<DevupComponentBaseProps<T>, DevupComponentProps<T>>,
): React.ReactElement
export function Text<T extends React.ElementType = 'span'>(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: Merge<DevupComponentBaseProps<T>, DevupComponentProps<T>>,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
