/// <reference types="@devup-ui/react/compat/css-prop" />

declare module '@emotion/styled' {
  import { styled } from '@devup-ui/react'

  export default styled
}

declare module '@emotion/react' {
  export { css, keyframes } from '@devup-ui/react'
  export type { StyledTheme as Theme } from '@devup-ui/react/compat'
  export {
    jsx as createElement,
    Global,
    jsx,
    ThemeProvider,
    useTheme,
    withTheme,
  } from '@devup-ui/react/compat'
}
