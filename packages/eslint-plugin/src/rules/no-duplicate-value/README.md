# no-duplicate-value

Disallow consecutive duplicate values in arrays within devup-ui components and utilities.

## Rule Details

This rule prevents the use of consecutive duplicate literal values in arrays that are passed to devup-ui components or utilities. Consecutive duplicate values are considered redundant since they don't provide any additional styling information and can be replaced with `null` values.

### Examples

#### ❌ Incorrect

```tsx
import { Box } from '@devup-ui/react'

// Consecutive duplicate values are redundant
;<Box w={[1, 1, 1]} />
;<Box w={[1, 2, 2, 3]} />
```

```tsx
import { css } from '@devup-ui/react'

// Consecutive duplicates in css utility
css({ w: [1, 2, 2, 2, 3] })
```

#### ✅ Correct

```tsx
import { Box } from '@devup-ui/react'

// No consecutive duplicates
;<Box w={[1, 2, 3]} />
;<Box w={[1, null, null]} /> // null values are fine
;<Box w={[1, 2, null, 3]} /> // null between different values
```

```tsx
import { Box } from 'other-package'

// Only applies to devup-ui components
;<Box w={[1, 1, 1]} />
```

```tsx
import { css } from '@devup-ui/react'

// No consecutive duplicates
css({ w: [1, 2, 3] })
css({ w: [1, 2, null, null, 3] })
```

## When Not To Use It

This rule is specifically designed for devup-ui components and utilities. It only applies when:

- Using devup-ui components (e.g., `Box`, `Flex`, etc.)
- Using devup-ui utilities (e.g., `css` function)
- The array contains consecutive literal values that are identical

The rule will not trigger for:

- Non-consecutive duplicate values (e.g., `[1, 2, 1]`)
- Arrays used with other libraries
- Non-literal values
- Arrays that are part of member expressions

## Auto-fixable

This rule is auto-fixable. ESLint will automatically replace consecutive duplicate values with `null` when possible.
