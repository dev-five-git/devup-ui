# Responsive

Devup UI supports responsive design.

# Breakpoints

Devup UI provides flexible responsive design capabilities through breakpoints. You can use either the default breakpoint system or customize your own breakpoints to match your design requirements.

## Default Breakpoints

By default, Devup UI uses a 5-breakpoint system with the following viewport ranges:

| Index | Viewport Range | Description     |
| ----- | -------------- | --------------- |
| 1st   | ~ 479px        | Mobile (small)  |
| 2nd   | 480px ~ 767px  | Mobile (large)  |
| 3rd   | 768px ~ 991px  | Tablet          |
| 4th   | 992px ~ 1279px | Desktop (small) |
| 5th   | 1280px ~       | Desktop (large) |

## Using Responsive Arrays

You can add responsive design by using an array of maximum 5 elements for any style property:

```jsx
const box = (
  <Box bg={["red", "blue", "green", "yellow", "purple"]} w={25} h={25}>
    <Text>Hello</Text>
  </Box>
);
```

### Null Values

If any of the five elements are set to `null`, Devup UI uses the previous value for responsive design:

```jsx
// Background will be:
// - red from 0px to 767px (1st and 2nd breakpoints)
// - green from 768px to 1279px (3rd and 4th breakpoints)
// - purple from 1280px and above (5th breakpoint)
<Box bg={["red", null, "green", null, "purple"]} />
```

### Partial Arrays

If the length of the array is less than 5, Devup UI makes responsive design according to the index of the element:

```jsx
// Only specify mobile and desktop values
<Box bg={['red', 'blue']} /> // red for mobile, blue for larger screens

// Specify mobile, tablet, and desktop
<Box bg={['red', 'blue', 'green']} />
```

## Custom Breakpoints

You can customize breakpoints by adding a `breakpoints` configuration to your `devup.json` file:

### Configuration

```json
{
  "theme": {
    "breakpoints": [0, 460, 768, 1024]
  }
}
```

### How Custom Breakpoints Work

When you define custom breakpoints, the array values correspond to minimum viewport widths:

- **1st element**: From 0px to the first breakpoint value
- **2nd element**: From first breakpoint to second breakpoint
- **3rd element**: From second breakpoint to third breakpoint
- **And so on...**

### Example with Custom Breakpoints

```json
// create devup.json
{
  "theme": {
    "breakpoints": [0, 460, 1024]
  }
}
```

```jsx
// With the custom breakpoints above:
<Box bg={["red", "blue", "green"]} />
```

This would create the following responsive behavior:

- `red` background from 0px to 459px
- `blue` background from 460px to 1023px
- `green` background from 1024px and above
