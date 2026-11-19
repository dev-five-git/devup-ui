// Type-checks a project against the published declarations (`dist`), as
// resolved through the package's exports: every entry resolves, and a theme
// augmentation reaches the types of every entry
import {
  Box,
  css,
  type DevupTheme,
  type DevupThemeTypographyKeys,
  setTheme,
  styled,
  stylex as namespace,
} from '@devup-ui/react'
import { ThemeProvider } from '@devup-ui/react/compat'
import * as stylex from '@devup-ui/react/stylex'

type Assert<T extends true> = T
type Equal<Left, Right> =
  (<T>() => T extends Left ? 1 : 2) extends <T>() => T extends Right ? 1 : 2
    ? true
    : false

export type Checks = [
  Assert<Equal<DevupThemeTypographyKeys, 'heading'>>,
  Assert<Equal<keyof DevupTheme, 'dark' | 'light'>>,
  Assert<Equal<typeof namespace.create, typeof stylex.create>>,
  Assert<Equal<typeof namespace.when, typeof stylex.when>>,
]

export const box = (
  <Box
    bg="$primary"
    boxShadow="$card"
    color="$gray.100"
    typography="heading"
    w="$containerX"
  />
)
// @ts-expect-error a typography the theme does not define
export const unknownTypography = <Box typography="nope" />
setTheme('dark')
// @ts-expect-error a theme the theme does not define
setTheme('sepia')
export const className = css({ typography: 'heading' })
// @ts-expect-error a typography the theme does not define
export const unknownClassName = css({ typography: 'nope' })
export const Title = styled.h1`
  color: red;
`
export const provider = <ThemeProvider theme={{ colors: { primary: 'red' } }} />
export const styles = stylex.create({ root: { color: 'red' } })
