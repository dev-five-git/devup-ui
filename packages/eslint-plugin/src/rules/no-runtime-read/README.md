# no-runtime-read

Disallow reading what the build compiles away anywhere but where it is called or rendered.

## Rule Details

The build replaces every call of `css`, `globalCss`, `keyframes` and `createGlobalStyle`, every `styled` component and every `Box`, `Flex`, `Text`... element with plain code and removes their imports. Code that reads one of them as a value is left reading something that no longer exists, and the build rejects it with ``X is read at runtime, where it does not exist``. The rule reports the same reads before the build does, with the same meaning.

It follows the names the build compiles: the imports from `@devup-ui/react`, and from `@emotion/react`, `@emotion/styled`, `styled-components` and `@vanilla-extract/css` through the import aliases the build compiles by default (`style` and `globalStyle` there are `css` and `globalCss`), and the module-level bindings that only alias them (`const style = css`).

Allowed reads:

- a call or a tagged template of a style function: `css({ ... })`, `` css`...` ``, `keyframes({ ... })`
- a `styled` chain that is built into a component: `styled.div({ ... })`, `` styled.div`...` ``, `styled('div')({ ... })`, `styled('div', { ... })`, `styled(Box)({ ... })`, `styled.div.attrs({ ... })({ ... })`
- an element: `<Box />`, and `as={Text}` on a Devup UI component
- a type: `typeof css`
- a module-level alias: `const alias = css`

### Examples

#### ❌ Incorrect

```tsx
import { Box, css, styled } from '@devup-ui/react'

export const runtime = css
export default [Box]
export const bare = styled.div
export const unfinished = styled(Box)
const withMember = css.foo
React.createElement(Box)
function f() {
  // Only a module-level alias is removed by the build
  const local = css
  return local({})
}
```

#### ✅ Correct

```tsx
import { Box, css, styled } from '@devup-ui/react'

export const className = css({ p: 1 })
export const Card = styled.div({ p: 1 })
export const Wrapped = styled(Box)({ p: 1 })
const alias = css
export const other = alias({ m: 1 })
export const element = <Box as="a" />
```

## When Not To Use It

The rule does nothing in vanilla-extract stylesheets (`.css.ts`, `.css.js`), which the build runs as they are, or for packages the build does not read as Devup UI (`@emotion/css`, for instance). Reads through the package imported whole (`Devup.css`) are left to the build.