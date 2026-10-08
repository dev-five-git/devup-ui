# prefer-media-shorthand

Prefer the media shorthand props over spelling out the same media query.

## Rule Details

`_media` entries and `'@media …'` keys whose query is exactly one of the
shorthands are reported. Whitespace and letter case in the query are ignored.
Keys the build does not read as styles — in a prop the component passes through
(`data-*`, `props`, ...) or in an argument of another function — are not
checked.

| Query                                    | Shorthand        |
| ---------------------------------------- | ---------------- |
| `all`                                    | `_all`           |
| `print`                                  | `_print`         |
| `screen`                                 | `_screen`        |
| `(prefers-reduced-motion: reduce)`       | `_motionReduce`  |
| `(prefers-reduced-motion: no-preference)` | `_motionSafe`    |
| `(orientation: portrait)`                | `_portrait`      |
| `(orientation: landscape)`               | `_landscape`     |
| `(prefers-contrast: more)`               | `_contrastMore`  |
| `(prefers-contrast: less)`               | `_contrastLess`  |
| `(forced-colors: active)`                | `_forcedColors`  |

### Examples

#### ❌ Incorrect

```tsx
import { Box, css } from '@devup-ui/react'

;<Box _media={{ '(prefers-reduced-motion: reduce)': { transition: 'none' } }} />
css({ '@media print': { color: 'black' } })
```

#### ✅ Correct

```tsx
import { Box, css } from '@devup-ui/react'

;<Box _motionReduce={{ transition: 'none' }} />
css({ _print: { color: 'black' } })
css({ _media: { '(min-width: 500px)': { p: 1 } } })
```

## Auto-fixable

A `_media` object holding only the matched query, and a static `'@media …'`
key, are rewritten to the shorthand. A `_media` object with other queries is
reported without a fix.


## Where it applies

The rule checks what the build reads as styles:

- the style props of `Box`, `Flex` and the other Devup UI components, and the arguments of `css`, `globalCss`, `keyframes` and `createGlobalStyle`
- the same calls through the import aliases the build compiles by default: `css`, `keyframes` and `createGlobalStyle` from `@emotion/react` and `styled-components`, `style` and `globalStyle` from `@vanilla-extract/css` (outside stylesheets), and `<Global styles={...}>` from `@emotion/react`
- the rules of `styled` from `@emotion/styled` and `styled-components`: `styled.div({ ... })`, `styled('div')({ ... })`, `styled(Base)({ ... }, { ... })` and `styled('div', { ... })`. The arguments of `.attrs()` and `.withConfig()`, and CSS text, are not styles
- a module-level `const` object or array that is not exported, is never changed and is read only as a style (`const s = { w: [1, 1] }; css(s)`), where it is declared

Nothing is checked in vanilla-extract stylesheets (`.css.ts`, `.css.js`): the build runs them as they are, and an array there is vanilla-extract's, not a responsive array.
