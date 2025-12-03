import type { DevupPropsWithTheme } from '../types/props'

interface StyledCreator {
  <T extends React.ElementType | React.ComponentType>(
    tag: T,
  ): (
    strings: TemplateStringsArray | DevupPropsWithTheme,
    ...values: (
      | ((props: React.ComponentProps<T>) => unknown)
      | string
      | number
      | boolean
      | null
      | undefined
    )[][]
  ) => (props: React.ComponentProps<T>) => React.ReactElement
}

type Styled = StyledCreator & {
  [T in keyof React.JSX.IntrinsicElements]: <P>(
    strings: TemplateStringsArray | DevupPropsWithTheme,
    ...values: (
      | ((props: P & React.ComponentProps<T>) => unknown)
      | string
      | number
      | boolean
      | null
      | undefined
    )[]
  ) => (props: P & React.ComponentProps<T>) => React.ReactElement
}

export const styled: Styled = new Proxy(Function.prototype, {
  get() {
    return () => {
      throw new Error('Cannot run on the runtime')
    }
  },
}) as any
