import type {
  DevupComponentMergedProps,
  DevupDefaultComponentMergedProps,
} from '../types/props'

export function Image(
  props: DevupDefaultComponentMergedProps<'img'>,
): React.ReactElement
export function Image<T extends React.ElementType>(
  props: DevupComponentMergedProps<T>,
): React.ReactElement
export function Image(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
