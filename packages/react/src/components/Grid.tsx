import type {
  DevupComponentMergedProps,
  DevupDefaultComponentMergedProps,
} from '../types/props'

export function Grid(
  props: DevupDefaultComponentMergedProps<'div'>,
): React.ReactElement
export function Grid<T extends React.ElementType>(
  props: DevupComponentMergedProps<T>,
): React.ReactElement
export function Grid(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
