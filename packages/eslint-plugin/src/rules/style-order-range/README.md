# style-order-range

Ensures `styleOrder` prop is within valid range (0 < value < 255).

## Rule Details

An explicit `styleOrder` must be an integer from 1 to 254, or the canonical
decimal string for that integer. Strings must not contain signs, whitespace,
leading zeros, decimal points, exponents or suffixes. Numeric expressions such
as `100.0` and `1e2` are integers and are accepted. The extractor reserves order
0 for its internal global layer and 255 for unlayered styles.

### Examples of **incorrect** code for this rule:

```jsx
import { Box } from '@devup-ui/react'

// Zero and negative values
<Box styleOrder={0} />
<Box styleOrder={-1} />
<Box styleOrder="-5" />

// Values greater than or equal to 255
<Box styleOrder={255} />
<Box styleOrder={256} />
<Box styleOrder="300" />

// Non-numeric values
<Box styleOrder="100px" />
<Box styleOrder="01" />
<Box styleOrder=" 1" />
<Box styleOrder="1e2" />
<Box styleOrder={1.5} />
<Box styleOrder={undefined} />
```

### Examples of **correct** code for this rule:

```jsx
import { Box } from '@devup-ui/react'

// Valid range values (0 < value < 255)
<Box styleOrder={1} />
<Box styleOrder={254} />
<Box styleOrder={1e2} />
<Box styleOrder="100" />
<Box styleOrder={active ? 1 : 2} />
<Box styleOrder={active && 2} />
```

## Where it applies

The rule checks Devup UI component props and top-level style-object keys of the
recognized utilities and styled factories, including their supported import
aliases. It does not inspect `.attrs()` data, pass-through props, native elements
or vanilla-extract stylesheets (`.css.ts`, `.css.js`). Conditional branches use
the same value contract. A native `<div styleOrder="100px" />` is not checked.

## When Not To Use It

If you don't use `styleOrder` props or want to allow any value range, you can disable this rule.
