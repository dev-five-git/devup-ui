/* eslint-disable @typescript-eslint/no-unused-expressions */
import { Box, css, styled } from '../src'
import { createGlobalStyle } from '../src/compat'

declare const active: boolean

// TOOL-14: the forms the build compiles on a styled component.
export const Attrs = styled.button.attrs({ type: 'button' })<{
  $active: boolean
}>`
  color: ${(props) => (props.$active ? 'red' : 'blue')};
`
export const FromProps = styled.div.attrs((props) => ({
  role: props.theme ? 'note' : undefined,
}))({ p: 1 })
export const Configured = styled.span
  .withConfig({
    shouldForwardProp: (prop) => prop !== '$active',
    displayName: 'Configured',
  })
  .attrs({ title: 'a' })`color: red;`
export const Rendered = styled('div')`
  color: red;
`.withComponent('aside')
export const OfBox = styled(Box).attrs({ p: 1 })({ m: 2 })
export const Plain = styled('div', { p: 1 })
export const Curried = styled.div({ bg: 'red', _hover: { bg: 'blue' } })
export const Global = createGlobalStyle`body { margin: ${0}px; }`
export const element = (
  <>
    <Attrs $active={active} />
    <Rendered />
    <Plain id="a" />
  </>
)
export const className = css({ p: 1 })

// @ts-expect-error the attrs of a tag are props of that tag
styled.div.attrs({ href: '/docs' })``
// @ts-expect-error `withConfig` takes the config object
styled.div.withConfig('name')``
// @ts-expect-error styles must be a rule object, CSS text or a condition between them
styled.div(() => ({ color: 'red' }))
// @ts-expect-error a component renders a tag or a component, not a number
styled(1)
// @ts-expect-error a global style takes no function interpolation: it is evaluated at build time
createGlobalStyle`body { color: ${(props: { foreground: string }) => props.foreground}; }`
