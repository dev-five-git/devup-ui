# no-typography-token-prefix

Disallow the `$` token prefix on `typography` values.

## Rule Details

`typography` takes the bare key of a `theme.typography` entry. Unlike color,
length, and shadow tokens it has no `$` prefix; `typography="$heading"` names a
preset that does not exist, so no style is applied.

The rule checks string values of `typography` on Devup UI components and
utilities, including values inside responsive arrays, conditionals, and
selector objects.

### Examples

#### ❌ Incorrect

```tsx
import { Box, css } from '@devup-ui/react'

;<Box typography="$heading" />
;<Box typography={['$body', null, '$heading']} />
css({ _hover: { typography: '$title' } })
```

#### ✅ Correct

```tsx
import { Box, css } from '@devup-ui/react'

;<Box typography="heading" />
;<Box color="$primary" />
css({ _hover: { typography: 'title' } })
```

## Auto-fixable

The `$` prefix is removed.
