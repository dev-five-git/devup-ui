# no-useless-tailing-nulls

Disallow useless trailing null values in arrays within devup-ui components and utilities.

## Rule Details

This rule prevents the use of trailing `null` values in arrays that are passed to devup-ui components or utilities. Trailing nulls are considered useless because they don't affect the styling behavior and can be safely removed.

### Examples

#### ❌ Incorrect

```tsx
import { Box } from '@devup-ui/react'

// Trailing nulls are useless
;<Box w={[1, 2, null]} />
;<Box w={[1, 2, null, null]} />
;<Box w={[null, null, null, null]} />
```

```tsx
import { css } from '@devup-ui/react'

// Trailing nulls in css utility
css({ w: [1, 2, null, null] })
```

#### ✅ Correct

```tsx
import { Box } from '@devup-ui/react'

// No trailing nulls
;<Box w={[1, 2]} />
;<Box w={[1, 2, null, 3]} /> // null in the middle is fine
;<Box w={[]} />
```

```tsx
import { Box } from 'other-package'

// Only applies to devup-ui components
;<Box w={[1, 2, null]} />
```

## When Not To Use It

This rule is specifically designed for devup-ui components and utilities. It only applies when:

- Using devup-ui components (e.g., `Box`, `Flex`, etc.)
- Using devup-ui utilities (e.g., `css` function)
- The array is not part of a member expression (e.g., `array[1]`)

The rule will not trigger for:

- Arrays with null values in the middle
- Arrays used with other libraries
- Arrays that are part of member expressions
- Arrays the build does not read as styles: in props the component passes through (`data-*`, `aria-*`, event handlers, HTML attributes, `props`, `styleVars`), in arguments of other functions, and under `imports`/`fontFaces`/`params`

## Auto-fixable

This rule is auto-fixable. ESLint will automatically remove trailing null values when possible.


## Where it applies

The rule checks what the build reads as styles:

- the style props of `Box`, `Flex` and the other Devup UI components, and the arguments of `css`, `globalCss`, `keyframes` and `createGlobalStyle`
- the same calls through the import aliases the build compiles by default: `css`, `keyframes` and `createGlobalStyle` from `@emotion/react` and `styled-components`, `style` and `globalStyle` from `@vanilla-extract/css` (outside stylesheets), and `<Global styles={...}>` from `@emotion/react`
- the rules of `styled` from `@emotion/styled` and `styled-components`: `styled.div({ ... })`, `styled('div')({ ... })`, `styled(Base)({ ... }, { ... })` and `styled('div', { ... })`. The arguments of `.attrs()` and `.withConfig()`, and CSS text, are not styles
- a module-level `const` object or array that is not exported, is never changed and is read only as a style (`const s = { w: [1, 1] }; css(s)`), where it is declared

Nothing is checked in vanilla-extract stylesheets (`.css.ts`, `.css.js`): the build runs them as they are, and an array there is vanilla-extract's, not a responsive array.
