declare module '@emotion/styled' {
  import { styled } from '@devup-ui/react'

  export default styled
}

declare module '@emotion/react' {
  export { css, keyframes } from '@devup-ui/react'
  export type { StyledTheme as Theme } from '@devup-ui/react/compat'
  export {
    Global,
    ThemeProvider,
    useTheme,
    withTheme,
  } from '@devup-ui/react/compat'
}
