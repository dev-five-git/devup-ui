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

type StyledProps<P, T extends React.ElementType | React.ComponentType> = P &
  React.ComponentProps<T>

/** What `.withConfig()` takes: the props to forward, and names the build ignores */
export interface StyledConfig {
  shouldForwardProp?: (
    prop: string,
    defaultValidatorFn: (prop: string) => boolean,
  ) => boolean
  displayName?: string
  componentId?: string
}

/** A component the build generates from `styled`, which can render another tag with the same styles */
export interface StyledComponent<P> {
  (props: P): React.ReactElement
  withComponent<U extends React.ElementType | React.ComponentType>(
    tag: U,
  ): StyledComponent<React.ComponentProps<U>>
}

/** What `styled.div`, `styled('div')` and what `.attrs()` and `.withConfig()` give: rules or CSS text come next */
export interface StyledTemplate<
  T extends React.ElementType | React.ComponentType,
  B = unknown,
> {
  <P = Record<string, unknown>>(
    strings: TemplateStringsArray | DevupPropsWithTheme,
    ...values: Interpolation<P & B, T>[]
  ): StyledComponent<P & B & StyledProps<unknown, T>>
  attrs(
    attrs:
      | Partial<StyledProps<B, T>>
      | ((props: InterpolationProps<B, T>) => Partial<StyledProps<B, T>>),
  ): StyledTemplate<T, B>
  withConfig(config: StyledConfig): StyledTemplate<T, B>
}

interface StyledCreator {
  <T extends React.ElementType | React.ComponentType>(
    tag: T,
    styles: DevupPropsWithTheme,
  ): StyledComponent<React.ComponentProps<T>>
  <T extends React.ElementType | React.ComponentType>(tag: T): StyledTemplate<T>
}

type Styled = StyledCreator & {
  [T in keyof React.JSX.IntrinsicElements]: StyledTemplate<T>
}

export const styled: Styled = new Proxy(Function.prototype, {
  get() {
    return () => {
      throw new Error('Cannot run on the runtime')
    }
  },
}) as unknown as Styled
