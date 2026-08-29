import type {
  DevupComponentMergedProps,
  DevupDefaultComponentMergedProps,
} from '../types/props'

export function Text(
  props: DevupDefaultComponentMergedProps<'span'>,
): React.ReactElement
export function Text<T extends React.ElementType>(
  props: DevupComponentMergedProps<T>,
): React.ReactElement
export function Text(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
