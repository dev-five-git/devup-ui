import type { DevupPropsWithTheme, StyledTheme } from '../types/props'

/**
 * Props an interpolation receives. `theme` is always present: `ThemeProvider`
 * publishes it as CSS variables, and the extractor resolves `props.theme.x`
 * reads to `var(--x)` at build time.
 */
type InterpolationProps<
  P,
  T extends React.ElementType | React.ComponentType,
> = P & React.ComponentProps<T> & { theme: StyledTheme }

type Interpolation<P, T extends React.ElementType | React.ComponentType> =
  | ((props: InterpolationProps<P, T>) => unknown)
  | string
  | number
  | boolean
  | null
  | undefined

interface StyledCreator {
  <T extends React.ElementType | React.ComponentType>(
    tag: T,
    styles: DevupPropsWithTheme,
  ): (props: React.ComponentProps<T>) => React.ReactElement
  <T extends React.ElementType | React.ComponentType>(
    tag: T,
  ): (
    strings: TemplateStringsArray | DevupPropsWithTheme,
    ...values: Interpolation<unknown, T>[][]
  ) => (props: React.ComponentProps<T>) => React.ReactElement
}

type Styled = StyledCreator & {
  [T in keyof React.JSX.IntrinsicElements]: <P = Record<string, unknown>>(
    strings: TemplateStringsArray | DevupPropsWithTheme,
    ...values: Interpolation<P, T>[]
  ) => (props: P & React.ComponentProps<T>) => React.ReactElement
}

export const styled: Styled = new Proxy(Function.prototype, {
  get() {
    return () => {
      throw new Error('Cannot run on the runtime')
    }
  },
}) as unknown as Styled
