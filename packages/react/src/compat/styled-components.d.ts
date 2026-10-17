declare module 'styled-components' {
  import { styled } from '@devup-ui/react'

  export { css, keyframes } from '@devup-ui/react'
  export type { StyledTheme as DefaultTheme } from '@devup-ui/react/compat'
  export {
    createGlobalStyle,
    createTheme,
    isStyledComponent,
    ServerStyleSheet,
    StyleSheetManager,
    ThemeConsumer,
    ThemeProvider,
    useTheme,
    withTheme,
  } from '@devup-ui/react/compat'

  export { styled }
  export default styled
}
