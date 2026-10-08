import { Box, css } from '../src'

declare module '../src' {
  interface DevupTheme {
    dark: null
  }
}

// TOOL-12: a custom property takes a number as well as a string.
export const vars = (
  <Box styleVars={{ '--count': 1, '--color': 'red', '--none': undefined }} />
)

// TOOL-09: the layers the build places a style in run from 1 to 254, and it ignores any other value.
export const order = (
  <>
    <Box styleOrder={1} />
    <Box styleOrder={254} />
    <Box styleOrder="100" />
  </>
)
css({ styleOrder: 20, p: 1 })

declare const dynamic: number
// @ts-expect-error 0 is below the first layer
export const zero = <Box styleOrder={0} />
// @ts-expect-error 255 is above the last layer
export const high = <Box styleOrder={255} />
// @ts-expect-error a value known only at runtime is ignored by the build
export const runtime = <Box styleOrder={dynamic} />
// @ts-expect-error a string that is not a layer number
export const junk = <Box styleOrder="100px" />
// @ts-expect-error a value known only at runtime is ignored by the build
css({ styleOrder: dynamic })

// custom shorthands written by hand take the responsive forms through `ResponsiveValue`
declare module '../src' {
  interface DevupCustomShorthands {
    insetY?: import('../src').ResponsiveValue<string | number>
  }
}
export const shorthand = (
  <Box _hover={{ insetY: 3 }} insetY={[1, null, 'auto']} />
)
// @ts-expect-error an object is not a value of the shorthand
export const badShorthand = <Box insetY={{ a: 1 }} />
