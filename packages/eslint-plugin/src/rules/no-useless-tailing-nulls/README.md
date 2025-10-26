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

## Auto-fixable

This rule is auto-fixable. ESLint will automatically remove trailing null values when possible.
