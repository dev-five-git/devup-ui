# css-utils-literal-only

Enforce that CSS utility functions only use values known at build time in devup-ui.

## Rule Details

This rule ensures that CSS utility functions (`css`, `globalCss`, `keyframes`) from devup-ui only receive values the build knows. They have no element to set a CSS variable on, so a value known only at runtime is a build error.

The build knows:

- literals, and constants: imports and module-level `const`s
- what those compute through calls of imported or module-level functions and of built-ins (`Math` except `Math.random`, `String`, `Number`, ...)

The build inlines constants, folds `Math` and runs the other calls at build time.

The rule reports parameters, `let` variables, `Date`, `Math.random` and functions the build cannot run, such as a parameter or a `let` function.

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

import { darken, PRIMARY } from './color'

// The build runs these calls
const HOVER = darken(0.1, PRIMARY)
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
- Constants, and what module functions, imports and built-ins compute from them
- Non-CSS utility functions

## Why This Rule Exists

CSS utilities in devup-ui compile to static classes at build time, so every value they read must be known then. The rule catches a value the build cannot know before the build does, at the line that reads it.
