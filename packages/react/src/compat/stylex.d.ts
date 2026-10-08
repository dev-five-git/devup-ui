declare module '@stylexjs/stylex' {
  import { stylex } from '@devup-ui/react'

  export type {
    PositionTryStyles,
    StylexDeclarations,
    ViewTransitionStyles,
  } from '@devup-ui/react/stylex'
  export {
    attrs,
    create,
    createTheme,
    createThemeContract,
    defineConsts,
    defineVars,
    firstThatWorks,
    include,
    keyframes,
    positionTry,
    props,
    types,
    viewTransitionClass,
  } from '@devup-ui/react/stylex'

  export default stylex
}
