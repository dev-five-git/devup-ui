import type {
  DevupDefaultComponentMergedProps,
  DevupPolymorphicComponentMergedProps,
} from '../types/props'

export function Flex(
  props: DevupDefaultComponentMergedProps<'div'>,
): React.ReactElement
export function Flex<T extends React.ElementType>(
  props: DevupPolymorphicComponentMergedProps<T>,
): React.ReactElement
export function Flex(
  props: DevupDefaultComponentMergedProps<'div'>,
): React.ReactElement
export function Flex(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
