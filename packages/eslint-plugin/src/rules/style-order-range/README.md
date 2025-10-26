# style-order-range

Ensures `styleOrder` prop is within valid range (0 < value < 255).

## Rule Details

This rule enforces that the `styleOrder` prop must be a number greater than 0 and less than 255.

### Examples of **incorrect** code for this rule:

```jsx
// Zero and negative values
<div styleOrder={0} />
<div styleOrder={-1} />
<div styleOrder="-5" />

// Values greater than or equal to 255
<div styleOrder={255} />
<div styleOrder={256} />
<div styleOrder="300" />

// Non-numeric values
<div styleOrder="abc" />
<div styleOrder={undefined} />
```

### Examples of **correct** code for this rule:

```jsx
// Valid range values (0 < value < 255)
<div styleOrder={1} />
<div styleOrder={254} />
<div styleOrder={128} />
<div styleOrder="100" />
<div styleOrder="1" />
<div styleOrder="254" />
```

## When Not To Use It

If you don't use `styleOrder` props or want to allow any value range, you can disable this rule.
