import {
  type Theme,
  ThemeProvider as EmotionThemeProvider,
  withTheme as withEmotionTheme,
} from '@emotion/react'
import type { ComponentProps, ReactNode, Ref, RefAttributes } from 'react'
import {
  type DefaultTheme,
  ThemeConsumer,
  ThemeProvider,
  withTheme,
} from 'styled-components'

type Accepts<Input, Contract> = [Input] extends [Contract] ? true : false
type Equal<Left, Right> =
  (<T>() => T extends Left ? 1 : 2) extends <T>() => T extends Right ? 1 : 2
    ? true
    : false
type RefProp<Props> = 'ref' extends keyof Props ? Props['ref'] : never

// Uses the module augmentations in compat.tsx, without adding new theme keys.
const styledTheme: DefaultTheme = { colors: { primary: 'red' } }
const emotionTheme: Theme = { spacing: { gap: 4 } }
type StyledProviderTheme = ComponentProps<typeof ThemeProvider>['theme']
type EmotionProviderTheme = ComponentProps<typeof EmotionThemeProvider>['theme']

export const styledObjectProvider = <ThemeProvider theme={styledTheme} />
export const emotionObjectProvider = (
  <EmotionThemeProvider theme={emotionTheme} />
)
export const styledFunctionProvider = (
  <ThemeProvider
    theme={(outer: DefaultTheme | undefined): DefaultTheme => ({
      colors: { primary: outer?.colors.primary ?? 'blue' },
    })}
  />
)
export const emotionFunctionProvider = (
  <EmotionThemeProvider
    theme={(outer: Theme | undefined): Theme => ({
      spacing: { gap: outer?.spacing.gap ?? 8 },
    })}
  />
)

export const styledRejectsNumber: Accepts<number, StyledProviderTheme> = false
export const emotionRejectsNumber: Accepts<number, EmotionProviderTheme> = false
export const styledRejectsCallbackResult: Accepts<
  (outer: DefaultTheme | undefined) => number,
  StyledProviderTheme
> = false
export const emotionRejectsCallbackResult: Accepts<
  (outer: Theme | undefined) => number,
  EmotionProviderTheme
> = false
export const styledRejectsRequiredOuter: Accepts<
  (outer: DefaultTheme) => DefaultTheme,
  StyledProviderTheme
> = false
export const emotionRejectsRequiredOuter: Accepts<
  (outer: Theme) => Theme,
  EmotionProviderTheme
> = false

export const styledConsumer = (
  <ThemeConsumer>{(theme) => <div>{theme.colors.primary}</div>}</ThemeConsumer>
)
type ConsumerTheme = Parameters<
  ComponentProps<typeof ThemeConsumer>['children']
>[0]
export const consumerKeepsAugmentation: Equal<ConsumerTheme, DefaultTheme> =
  true
export const consumerRejectsUndefined: Accepts<undefined, ConsumerTheme> = false

interface StyledOwnProps {
  readonly label: string
  readonly theme: DefaultTheme
}
interface EmotionOwnProps {
  readonly label: string
  readonly theme: Theme
}
function StyledView(props: StyledOwnProps): ReactNode {
  return <div>{props.label + props.theme.colors.primary}</div>
}
function EmotionView(props: EmotionOwnProps): ReactNode {
  return <div>{props.label + props.theme.spacing.gap}</div>
}
const StyledWithTheme = withTheme(StyledView)
const EmotionWithTheme = withEmotionTheme(EmotionView)
type StyledWrapperProps = ComponentProps<typeof StyledWithTheme>
type EmotionWrapperProps = ComponentProps<typeof EmotionWithTheme>
declare const forwardedRef: Ref<unknown>

export const styledInjected = <StyledWithTheme label="styled" />
export const emotionInjected = <EmotionWithTheme label="emotion" />
export const styledOverride = (
  <StyledWithTheme ref={forwardedRef} label="styled" theme={styledTheme} />
)
export const emotionOverride = (
  <EmotionWithTheme ref={forwardedRef} label="emotion" theme={emotionTheme} />
)
// The parent forwardRef returns RefAttributes<unknown>, not an inferred DOM ref.
export const styledKeepsParentRef: Equal<
  RefProp<StyledWrapperProps>,
  RefAttributes<unknown>['ref']
> = true
export const emotionKeepsParentRef: Equal<
  RefProp<EmotionWrapperProps>,
  RefAttributes<unknown>['ref']
> = true

export const styledRequiresOwnProp: Accepts<
  Record<never, never>,
  StyledWrapperProps
> = false
export const emotionRequiresOwnProp: Accepts<
  Record<never, never>,
  EmotionWrapperProps
> = false
export const styledRejectsOwnPropType: Accepts<
  { readonly label: number },
  StyledWrapperProps
> = false
export const emotionRejectsOwnPropType: Accepts<
  { readonly label: number },
  EmotionWrapperProps
> = false
export const styledRejectsThemeOverride: Accepts<
  { readonly label: string; readonly theme: number },
  StyledWrapperProps
> = false
export const emotionRejectsThemeOverride: Accepts<
  { readonly label: string; readonly theme: number },
  EmotionWrapperProps
> = false
export const styledRejectsRefNumber: Accepts<
  { readonly label: string; readonly ref: number },
  StyledWrapperProps
> = false
export const emotionRejectsRefNumber: Accepts<
  { readonly label: string; readonly ref: number },
  EmotionWrapperProps
> = false
