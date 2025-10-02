# css-utils-literal-only

Enforce that CSS utility functions only use literal values in devup-ui.

## Rule Details

This rule ensures that CSS utility functions (`css`, `globalCss`, `keyframes`) from devup-ui only receive literal values as property values. Using variables or expressions in CSS utilities can lead to runtime issues and prevents proper static analysis and optimization.

### Examples

#### ❌ Incorrect

```tsx
import { css } from '@devup-ui/react'

const v = 'some-value'

// Variables are not allowed in CSS utilities
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

const animationName = 'fade'

// Variables in keyframes are not allowed
keyframes({ from: { opacity: animationName } })
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
- Non-CSS utility functions

## Why This Rule Exists

CSS utilities in devup-ui are designed to work with static, literal values for optimal performance and build-time optimization. Using variables or dynamic expressions can:

- Prevent proper static analysis
- Cause runtime errors
- Reduce build-time optimizations
- Make the code harder to understand and maintain
