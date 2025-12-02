import type { DevupPropsWithTheme } from '../types/props'

interface StyledCreator {
  <T extends React.ElementType | React.ComponentType>(
    tag: T,
  ): (
    strings: TemplateStringsArray | DevupPropsWithTheme,
  ) => (props: React.ComponentProps<T>) => React.ReactElement
}

type Styled = StyledCreator & {
  [T in keyof React.JSX.IntrinsicElements]: <P extends React.ComponentProps<T>>(
    strings: TemplateStringsArray | DevupPropsWithTheme,
  ) => (props: P) => React.ReactElement
}

export const styled: Styled = new Proxy(Function.prototype, {
  get() {
    return () => {
      throw new Error('Cannot run on the runtime')
    }
  },
}) as any
