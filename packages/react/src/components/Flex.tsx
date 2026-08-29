import type {
  DevupComponentMergedProps,
  DevupDefaultComponentMergedProps,
} from '../types/props'

export function Flex(
  props: DevupDefaultComponentMergedProps<'div'>,
): React.ReactElement
export function Flex<T extends React.ElementType>(
  props: DevupComponentMergedProps<T>,
): React.ReactElement
export function Flex(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  props: unknown,
): React.ReactElement {
  throw new Error('Cannot run on the runtime')
}
