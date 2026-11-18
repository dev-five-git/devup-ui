# no-useless-fallback

Warn about a property's array in Emotion or vanilla-extract rules whose earlier values never apply.

## Rule Details

In the rules of Emotion (`@emotion/react`, `@emotion/styled`, `@emotion/css`) and vanilla-extract (`@vanilla-extract/css`), an array is not a responsive array: the build declares each value in turn, as these libraries do, so `margin: ['1px', '3px']` becomes `margin:1px;margin:3px`. A browser keeps the last declaration it understands, so an earlier value only matters when a later one may not be supported, as in `height: ['100vh', '100dvh']`.

This rule reports an array when every value is a string every browser supports: only the last value ever applies. Most often a responsive array was meant; Devup UI's `css()` and components such as `<Box>` read arrays as breakpoints.

The rule only reports when it is sure. It leaves an array alone when:

- a value is not a string written out (`3`, `size`, `` `${size}px` ``, `null`, a hole, a spread) or is empty;
- a value uses syntax that some browsers in use do not support yet, from this deliberately small list:
  - `var()` and `env()`, whose value is only known in the browser
  - vendor prefixes: `-webkit-`, `-moz-`, `-ms-`, `-o-`
  - small, large and dynamic viewport units (`dvh`, `svw`, `lvmin`...), container query units (`cqw`, `cqmin`...) and line height units (`lh`, `rlh`)
  - `oklch()`, `oklab()`, `lab()`, `lch()`, `hwb()`, `color()`, `color-mix()`, `light-dark()`, and `rgb()`/`hsl()` written without commas (`rgb(255 0 0 / 50%)`)
  - `clamp()`, `min()`, `max()`
- the key is a Devup UI shorthand (`p`, `bg`, `w`...), which reads an array as breakpoints in every library's rules, or holds no CSS value (`_hover`, `&:hover`, `@media ...`, `typography`, `selectors`);
- the rules are styled-components', which writes the values joined by commas as one declaration (`fontFamily: ['Arial', 'sans-serif']`), not fallbacks.

There is no autofix: removing the earlier values, or turning the array into a responsive one, changes what the build renders, so the choice is left to you. Shorthands defined in `devup.json` are not known to the rule.

### Examples

#### ❌ Incorrect

```tsx
import { css } from '@emotion/react'

css({ margin: ['1px', '3px'] }) // only 3px applies
css({ color: ['red', 'blue'] }) // only blue applies
```

```ts
import { style } from '@vanilla-extract/css'

style({ display: ['block', 'grid'] })
```

#### ✅ Correct

```tsx
import { css } from '@emotion/react'

css({ height: ['100vh', '100dvh'] }) // 100vh where dvh is not supported
css({ color: ['red', 'oklch(0.6 0.2 250)'] })
css({ display: ['-webkit-box', 'flex'] })
css({ margin: '3px' })
```

```tsx
import { Box, css } from '@devup-ui/react'

// Responsive values: a value per breakpoint
css({ margin: ['1px', '3px'] })
;<Box m={['1px', '3px']} />
```

Nothing is checked in vanilla-extract stylesheets (`.css.ts`, `.css.js`): the build runs them as they are.
