declare module 'styled-components' {
  import { styled } from '@devup-ui/react'

  export { css, keyframes } from '@devup-ui/react'
  export type { StyledTheme as DefaultTheme } from '@devup-ui/react/compat'
  export {
    createGlobalStyle,
    isStyledComponent,
    ServerStyleSheet,
    StyleSheetManager,
    ThemeProvider,
    useTheme,
    withTheme,
  } from '@devup-ui/react/compat'

  export { styled }
  export default styled
}
