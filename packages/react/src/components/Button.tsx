import type {
  DevupComponentMergedProps,
  DevupDefaultComponentMergedProps,
} from '../types/props'

export function Button(
  props: DevupDefaultComponentMergedProps<'button'>,
): React.ReactElement
export function Button<T extends React.ElementType>(
  props: DevupComponentMergedProps<T>,
): React.ReactElement
export function Button(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
