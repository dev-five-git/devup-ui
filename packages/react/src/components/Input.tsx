import type {
  DevupComponentMergedProps,
  DevupDefaultComponentMergedProps,
} from '../types/props'

export function Input(
  props: DevupDefaultComponentMergedProps<'input'>,
): React.ReactElement
export function Input<T extends React.ElementType>(
  props: DevupComponentMergedProps<T>,
): React.ReactElement
export function Input(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
