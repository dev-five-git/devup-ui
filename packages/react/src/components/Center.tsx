import type {
  DevupComponentMergedProps,
  DevupDefaultComponentMergedProps,
} from '../types/props'

export function Center(
  props: DevupDefaultComponentMergedProps<'div'>,
): React.ReactElement
export function Center<T extends React.ElementType>(
  props: DevupComponentMergedProps<T>,
): React.ReactElement
export function Center(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
