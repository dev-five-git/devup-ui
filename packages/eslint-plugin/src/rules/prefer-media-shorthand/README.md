# prefer-media-shorthand

Prefer the media shorthand props over spelling out the same media query.

## Rule Details

`_media` entries and `'@media …'` keys whose query is exactly one of the
shorthands are reported. Whitespace and letter case in the query are ignored.

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
