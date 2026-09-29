# css-utils-literal-only

Enforce that CSS utility functions only use values known at build time in devup-ui.

## Rule Details

This rule ensures that CSS utility functions (`css`, `globalCss`, `keyframes`, `createGlobalStyle`) from devup-ui, and the StyleX functions devup-ui compiles (`create`, `keyframes`, `defineVars`, `defineConsts`, `createTheme`, `createThemeContract`, `positionTry`, `viewTransitionClass`), only receive values the build knows. They have no element to set a CSS variable on, so a value known only at runtime is a build error.

It checks the values of every rule object they take, in any argument, and the interpolations of CSS text, written as a template argument or a tagged template (`` css`color: ${color};` ``). A part `css()` composes as a class (`css(base, { m: 1 })`), and a condition choosing between parts, are read at runtime and not checked. `styled()` sets a CSS variable on the element it renders, so its values are not checked either.

The build knows:

- literals, and constants: imports and module-level `const`s
- what those compute through exact built-ins (`String`, `Number`, `JSON`, string and array methods, `Math.max`, `Math.round`, ... — not `Math.random`, `Math.sin` or `Math.pow`) and through functions this file declares that only compute
- what StyleX functions give: `defineVars()` variables, `keyframes()` names, `firstThatWorks()`

The build inlines constants, folds `Math` and runs the file's own functions at build time. It never runs another module's code, so calling an imported function is reported.

The rule reports parameters, `let` variables, globals such as `Date` or `window`, `Math.random` and approximate `Math` functions (also through an alias such as `const { sin } = Math`), `**`, `toString` other than called at once without a radix, locale methods (`toLocaleString`, `localeCompare`, `normalize`), and functions the build does not run: imported ones, and those using `this`, `new`, classes, regular expressions, `try`, getters, `async` code, generators, JSX or a method chosen at runtime, or writing a module-level binding. It also reports a constant or an import the file changes — a member assignment, `delete`, `++`, a changing method such as `push` or `sort`, `Object.assign`, changing its elements in a `for...of` loop or a `forEach`/`map` callback, calling a method of it that uses `this`, or handing it to a function the rule does not know — as the build no longer reads it as written.

A lint rule sees one file, so two cases are left to the build: code in another module changing an imported object, and an import whose module computes its value (`export const DARK = darken(PRIMARY)`), which the build does not run. Keep objects styles read unchanged, and export the values styles read as literals.

### Examples

#### ❌ Incorrect

```tsx
import { css } from '@devup-ui/react'

let v = 'some-value'

// Values that can change are not allowed in CSS utilities
css({ w: v })
css({ w: [v] })
css({ w: [1, null, v] })
```

```tsx
import { globalCss } from '@devup-ui/react'

const dynamicValue = getValue()

// Dynamic values are not allowed
globalCss({ color: dynamicValue })
```

```tsx
import { keyframes } from '@devup-ui/react'

function fade(opacity: number) {
  // A parameter is only known at runtime
  return keyframes({ from: { opacity } })
}
```

```tsx
import { css } from '@devup-ui/react'

// `Date` and `Math.random` differ on every build
css({ w: Date.now(), h: Math.random() })

function tint(pick: (n: number) => string) {
  // The build cannot run a function it is given at runtime
  return css({ color: pick(1) })
}
```

```tsx
import { css } from '@devup-ui/react'

import { darken, PRIMARY } from './color'

// Another module's code never runs at build time
css({ color: darken(0.1, PRIMARY) })
```

```tsx
import { css } from '@devup-ui/react'

const sizes = { gap: 4 }

// Changed after it is declared, so it is not a constant
sizes.gap = 8
css({ gap: sizes.gap })
```

#### ✅ Correct

```tsx
import { css } from '@devup-ui/react'

// Only literal values are allowed
css({ w: 1 })
css({ w: '1' })
css({ w: [1] })
css({ w: ['1'] })
css({ w: [1, null, '2'] })
```

```tsx
import { globalCss } from '@devup-ui/react'

// Literal values only
globalCss({ color: 'red' })
globalCss({ fontSize: 16 })
```

```tsx
import { keyframes } from '@devup-ui/react'

// Literal values in keyframes
keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })
```

```tsx
import { css } from '@devup-ui/react'

import { SIZE } from './tokens'

// The build inlines constants: an import, or a module-level `const` whose
// value is a literal, a template or arithmetic over constants
const GAP = SIZE * 2
const UNIT = `${GAP}px`
css({ p: UNIT, m: SIZE })
```

```tsx
import { css } from '@devup-ui/react'

import { PRIMARY } from './color'

// The build runs the functions of this file and exact built-ins
const HOVER = `${PRIMARY}cc`
function double(n: number) {
  return n * 2
}
css({ color: HOVER, m: double(2), w: Math.max(4, 8), h: String(10) })

// A callback written in the value runs with it
css({ gap: [1, 2].map((n) => n * 4)[1] })
```

```tsx
import { css } from 'other-package'

// Only applies to devup-ui CSS utilities
css({ w: v }) // This is fine for other packages
```

## When Not To Use It

This rule is specifically designed for devup-ui CSS utility functions. It only applies when:

- Using devup-ui CSS utilities (`css`, `globalCss`, `keyframes`)
- The utility function is called with an object containing properties
- Property values contain variables or expressions

The rule will not trigger for:

- CSS utilities from other packages
- Literal values (strings, numbers, arrays of literals)
- Constants, and what exact built-ins and the file's own functions compute from them
- Non-CSS utility functions

## Why This Rule Exists

CSS utilities in devup-ui compile to static classes at build time, so every value they read must be known then. The rule catches a value the build cannot know before the build does, at the line that reads it.
