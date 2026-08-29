import type {
  DevupDefaultComponentMergedProps,
  DevupPolymorphicComponentMergedProps,
} from '../types/props'

export function Button(
  props: DevupDefaultComponentMergedProps<'button'>,
): React.ReactElement
export function Button<T extends React.ElementType>(
  props: DevupPolymorphicComponentMergedProps<T>,
): React.ReactElement
export function Button(
  props: DevupDefaultComponentMergedProps<'button'>,
): React.ReactElement
export function Button(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
