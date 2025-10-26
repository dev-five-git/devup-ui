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

## Auto-fixable

This rule is auto-fixable. ESLint will automatically convert single-element arrays to their direct values when possible.
