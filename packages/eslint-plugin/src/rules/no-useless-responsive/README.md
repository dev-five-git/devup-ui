# no-useless-responsive

Disallow useless responsive arrays with single values in devup-ui components and utilities.

## Rule Details

This rule prevents the use of arrays with only one element when passed to devup-ui components or utilities. Single-element arrays are considered useless for responsive design since they don't provide any responsive behavior and can be simplified to just the value itself.

### Examples

#### ❌ Incorrect

```tsx
import { Box } from '@devup-ui/react'

// Single-element arrays are useless
;<Box w={[1]} />
;<Box w={[1]} />
```

```tsx
import { css } from '@devup-ui/react'

// Single-element arrays in css utility
css({ w: [1] })
```

```tsx
import { css as c } from '@devup-ui/react'

// Works with aliased imports
c({ w: [1] })
```

#### ✅ Correct

```tsx
import { Box } from '@devup-ui/react'

// Use the value directly instead of wrapping in array
;<Box w={1} />
;<Box w="1" />
;<Box w={[]} /> // Empty arrays are fine
```

```tsx
import { Box } from 'other-package'

// Only applies to devup-ui components
;<Box w={[1]} />
;<Box w={[1, 2, 3]} /> // Multi-element arrays are fine
```

```tsx
import { css } from '@devup-ui/react'

// Use the value directly
css({ w: 1 })
css({ w: '1' })
```

## When Not To Use It

This rule is specifically designed for devup-ui components and utilities. It only applies when:

- Using devup-ui components (e.g., `Box`, `Flex`, etc.)
- Using devup-ui utilities (e.g., `css` function)
- The array contains exactly one element

The rule will not trigger for:

- Arrays with multiple elements (e.g., `[1, 2, 3]`)
- Empty arrays (e.g., `[]`)
- Arrays used with other libraries
- Non-array values
- Arrays the build does not read as styles: in props the component passes through (`data-*`, `aria-*`, event handlers, HTML attributes, `props`, `styleVars`), in arguments of other functions, and under `imports`/`fontFaces`/`params`

## Auto-fixable

This rule is auto-fixable. ESLint will automatically convert single-element arrays to their direct values when possible.


## Where it applies

The rule checks what the build reads as styles:

- the style props of `Box`, `Flex` and the other Devup UI components, and the arguments of `css`, `globalCss`, `keyframes` and `createGlobalStyle`
- the same calls through the import aliases the build compiles by default: `css`, `keyframes` and `createGlobalStyle` from `@emotion/react` and `styled-components`, `style` and `globalStyle` from `@vanilla-extract/css` (outside stylesheets), and `<Global styles={...}>` from `@emotion/react`
- the rules of `styled` from `@emotion/styled` and `styled-components`: `styled.div({ ... })`, `styled('div')({ ... })`, `styled(Base)({ ... }, { ... })` and `styled('div', { ... })`. The arguments of `.attrs()` and `.withConfig()`, and CSS text, are not styles
- a module-level `const` object or array that is not exported, is never changed and is read only as a style (`const s = { w: [1, 1] }; css(s)`), where it is declared

Nothing is checked in vanilla-extract stylesheets (`.css.ts`, `.css.js`): the build runs them as they are, and an array there is vanilla-extract's, not a responsive array.
