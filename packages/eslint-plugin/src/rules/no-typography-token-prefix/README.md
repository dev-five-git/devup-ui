# no-typography-token-prefix

Disallow the `$` token prefix on `typography` values.

## Rule Details

`typography` takes the bare key of a `theme.typography` entry. Unlike color,
length, and shadow tokens it has no `$` prefix; `typography="$heading"` names a
preset that does not exist, so no style is applied.

The rule checks string values of `typography` on Devup UI components and
utilities, including values inside responsive arrays, conditionals, and
selector objects. A `typography` key the build does not read as a style — in a
prop the component passes through (`data-*`, `props`, ...) or in an argument of
another function — is not checked.

### Examples

#### ❌ Incorrect

```tsx
import { Box, css } from '@devup-ui/react'

;<Box typography="$heading" />
;<Box typography={['$body', null, '$heading']} />
css({ _hover: { typography: '$title' } })
```

#### ✅ Correct

```tsx
import { Box, css } from '@devup-ui/react'

;<Box typography="heading" />
;<Box color="$primary" />
css({ _hover: { typography: 'title' } })
```

## Auto-fixable

The `$` prefix is removed.


## Where it applies

The rule checks what the build reads as styles:

- the style props of `Box`, `Flex` and the other Devup UI components, and the arguments of `css`, `globalCss`, `keyframes` and `createGlobalStyle`
- the same calls through the import aliases the build compiles by default: `css`, `keyframes` and `createGlobalStyle` from `@emotion/react` and `styled-components`, `style` and `globalStyle` from `@vanilla-extract/css` (outside stylesheets), and `<Global styles={...}>` from `@emotion/react`
- the rules of `styled` from `@emotion/styled` and `styled-components`: `styled.div({ ... })`, `styled('div')({ ... })`, `styled(Base)({ ... }, { ... })` and `styled('div', { ... })`. The arguments of `.attrs()` and `.withConfig()`, and CSS text, are not styles
- a module-level `const` object or array that is not exported, is never changed and is read only as a style (`const s = { w: [1, 1] }; css(s)`), where it is declared

Nothing is checked in vanilla-extract stylesheets (`.css.ts`, `.css.js`): the build runs them as they are, and an array there is vanilla-extract's, not a responsive array.
