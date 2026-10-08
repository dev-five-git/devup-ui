/* eslint-disable @eslint-react/rules-of-hooks, react-hooks/rules-of-hooks, no-constant-binary-expression */
import {
  css as emotionCss,
  Global,
  type Keyframes as EmotionKeyframes,
  keyframes as emotionKeyframes,
  type SerializedStyles,
  type Theme,
  ThemeProvider as EmotionThemeProvider,
  useTheme as useEmotionTheme,
  withTheme as withEmotionTheme,
} from '@emotion/react'
import emotionStyled from '@emotion/styled'
import {
  assignVars,
  createGlobalTheme,
  createGlobalThemeContract,
  createTheme,
  createThemeContract,
  createVar,
  fontFace,
  globalFontFace,
  globalStyle,
  keyframes as vanillaKeyframes,
  style,
  styleVariants,
} from '@vanilla-extract/css'
import styled, {
  createGlobalStyle,
  css as styledCss,
  type CSSKeyframes,
  type CSSObject,
  type CSSProp,
  type CSSProperties,
  type CSSPseudos,
  type DefaultTheme,
  type Keyframes,
  keyframes as styledKeyframes,
  type RuleSet,
  ServerStyleSheet,
  StyleSheetManager,
  ThemeConsumer,
  ThemeContext,
  ThemeProvider,
  useTheme,
  withTheme,
} from 'styled-components'

declare module 'styled-components' {
  interface DefaultTheme {
    colors: { primary: string }
  }
}
declare module '@emotion/react' {
  interface Theme {
    spacing: { gap: number }
  }
}

// SC-16: the theme is an interface, so it is augmented as in styled-components.
const styledTheme: DefaultTheme = useTheme()
export const themed = styledTheme.colors.primary
// @ts-expect-error a key the augmentation does not declare is still a theme key, so a number is not a theme value
export const badThemeKey: DefaultTheme['colors'] = 1
export const emotionTheme: Theme = useEmotionTheme()
export const gap = emotionTheme.spacing.gap
export const object: CSSObject = { p: 1, _hover: { bg: 'red' } }
export const properties: CSSProperties = { color: 'red' }
export const pseudos: CSSPseudos = { ':hover': { p: 1 } }
export const frames: CSSKeyframes = { from: { opacity: 0 } }
export const prop: CSSProp = [{ p: 1 }, false, 'a']
export const consumer = ThemeConsumer
export const context = ThemeContext

// TOOL-13: each library names what its `css` and `keyframes` give.
const serialized: SerializedStyles = emotionCss({ color: 'red' })
const emotionFrames: EmotionKeyframes = emotionKeyframes({
  from: { opacity: 0 },
})
const rules: RuleSet = styledCss`color: red;`
const frameName: Keyframes = styledKeyframes`from { opacity: 0; }`
export const animation = emotionCss({ animationName: emotionFrames })
export const composed = emotionCss(serialized, 'extra', [{ p: 1 }], null)
export const sc = styledCss(rules, { p: 1 })
export const fromString: string = serialized
export const named = [frameName, emotionFrames, rules]
// @ts-expect-error a plain string is not what `css` gives
export const notSerialized: SerializedStyles = 'color: red;'
// @ts-expect-error a plain string is not what `keyframes` gives
export const notFrames: Keyframes = 'fade'
// @ts-expect-error an Emotion result is not a styled-components rule set
export const crossed: RuleSet = serialized

// TOOL-14 / TOOL-15: the build compiles these, and rejects the rest.
export const Button = styled.button.attrs({ type: 'button' })<{
  active: boolean
}>`
  color: ${(props) => (props.active ? 'red' : 'blue')};
`
export const EmotionButton = emotionStyled.div({ p: 1 })
export const GlobalStyle = createGlobalStyle`body { margin: ${0}px; }`
export const WithTheme = withTheme((props: { theme?: DefaultTheme }) => (
  <div>{props.theme?.colors.primary}</div>
))
export const EmotionWithTheme = withEmotionTheme((props: { theme?: Theme }) => (
  <div>{props.theme?.spacing.gap}</div>
))
export const provider = (
  <ThemeProvider theme={{ colors: { primary: 'red' } }}>
    <div />
  </ThemeProvider>
)
export const emotionProvider = (
  <EmotionThemeProvider theme={{ spacing: { gap: 4 } }}>
    <Global styles={{ body: { margin: 0 } }} />
  </EmotionThemeProvider>
)
export const sheet = new ServerStyleSheet().collectStyles(<div />)
export const managed = (
  <StyleSheetManager>
    <div />
  </StyleSheetManager>
)
const outerTheme = (theme: DefaultTheme) => theme
const emotionOuterTheme = (theme: Theme) => theme
// @ts-expect-error an outer-theme function cannot be compiled: the theme is published as CSS variables
export const outer = <ThemeProvider theme={outerTheme} />
// @ts-expect-error an outer-theme function cannot be compiled: the theme is published as CSS variables
export const emotionOuter = <EmotionThemeProvider theme={emotionOuterTheme} />
// @ts-expect-error a function giving the styles is read at runtime
export const FunctionStyles = emotionStyled.div(() => ({ color: 'red' }))

// TOOL-16 / VE-06: what the evaluator registers.
export const composedStyle = style([
  { color: 'red' },
  { padding: 4 },
  'external',
])
export const nested = style([[{ color: 'red' }], false && { margin: 0 }])
export const single = style({ color: 'red' })
export const variants = styleVariants({
  red: { color: 'red' },
  both: [{ p: 1 }, 'a'],
})
export const mapped = styleVariants({ a: 1, b: 2 }, (value, key) => ({
  order: value,
  '--key': key,
}))
export const variable = createVar()
const contract = createThemeContract({ color: null })
export const themeTuple = createTheme(contract, { color: 'red' })
export const themeClass = createTheme({ color: 'red' })
export const assigned = assignVars(contract, { color: 'red' })
export const globalContract = createGlobalThemeContract(
  { color: 'color' },
  (value) => `prefix-${String(value)}`,
)
export const globalTheme = createGlobalTheme(':root', { color: 'red' })
createGlobalTheme(':root', contract, { color: 'red' })
export const family = fontFace({ src: 'url(font.woff2)' })
globalStyle('body', { margin: 0 })
globalFontFace('Inter', { src: 'url(font.woff2)' })
export const vanillaAnimation = vanillaKeyframes({ from: { opacity: 0 } })

// @ts-expect-error the evaluator registers no `globalKeyframes`
export { globalKeyframes } from '@vanilla-extract/css'
// @ts-expect-error the evaluator registers no `composeStyles`
export { composeStyles } from '@vanilla-extract/css'
// @ts-expect-error the evaluator registers no `generateIdentifier`
export { generateIdentifier } from '@vanilla-extract/css'
