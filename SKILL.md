---
name: devup-ui
description: |
  Zero-runtime CSS-in-JS preprocessor for React. Transforms JSX styles to static CSS at build time.

  TRIGGER WHEN:
  - Writing/modifying Devup UI components (Box, Flex, Grid, Text, Button, etc.)
  - Using styling APIs: css(), styled(), globalCss(), keyframes()
  - Configuring devup.json theme (colors, typography)
  - Setting up build plugins (Vite, Next.js, Webpack, Rsbuild, Bun)
  - Debugging "Cannot run on the runtime" errors
  - Working with responsive arrays or pseudo-selectors (_hover, _dark, etc.)
---

# Devup UI

Build-time CSS extraction. No runtime JS for styling.

## Critical: Components Are Compile-Time Only

All `@devup-ui/react` components (`Box`, `Flex`, `Text`, etc.) throw `Error('Cannot run on the runtime')`. They are **placeholders** that build plugins transform to `<div className="...">`.

```tsx
// BEFORE BUILD (what you write):
<Box bg="red" p={4} _hover={{ bg: "blue" }} />

// AFTER BUILD (what runs in browser):
<div className="a b c" />  // + CSS: .a{background:red} .b{padding:16px} .c:hover{background:blue}
```

## Style Prop Syntax

### Shorthands (ALWAYS use these)

| Short | Full | Short | Full |
|-------|------|-------|------|
| `bg` | background | `m`, `mt`, `mr`, `mb`, `ml`, `mx`, `my` | margin-* |
| `p`, `pt`, `pr`, `pb`, `pl`, `px`, `py` | padding-* | `w`, `h` | width, height |
| `minW`, `maxW`, `minH`, `maxH` | min/max width/height | `gap` | gap |

### Spacing Scale (× 4 = px)

```tsx
<Box p={1} />    // padding: 4px
<Box p={4} />    // padding: 16px
<Box p="4" />    // padding: 16px (unitless string also × 4)
<Box p="20px" /> // padding: 20px (with unit = exact value)
```

### Responsive Arrays (5 breakpoints)

```tsx
// [mobile, mid, tablet, mid, PC] - 5 levels
// Use indices 0, 2, 4 most frequently. Use null to skip.

<Box bg={["red", null, "blue", null, "yellow"]} />  // mobile=red, tablet=blue, PC=yellow
<Box p={[2, null, 4, null, 6]} />                   // mobile=8px, tablet=16px, PC=24px
<Box w={["100%", null, "50%"]} />                   // mobile=100%, tablet+=50%
```

### Pseudo-Selectors (underscore prefix)

```tsx
<Box
  _hover={{ bg: "blue" }}
  _focus={{ outline: "2px solid blue" }}
  _active={{ bg: "darkblue" }}
  _dark={{ bg: "gray.800" }}   // theme variant
  _before={{ content: '""' }}
  _firstChild={{ mt: 0 }}
/>
```

### Dynamic Values = CSS Variables

```tsx
// Static value -> class
<Box bg="red" />  // className="a" + .a{background:red}

// Dynamic value -> CSS variable
<Box bg={props.color} />  // className="a" style={{"--a":props.color}} + .a{background:var(--a)}

// Conditional -> preserved
<Box bg={isActive ? "blue" : "gray"} />  // className={isActive ? "a" : "b"}
```

## Styling APIs

```tsx
import { css, styled, globalCss, keyframes } from "@devup-ui/react";

// Reusable style object
const cardStyles = css({ bg: "white", p: 4, borderRadius: "8px" });
<Box {...cardStyles} />

// Styled component (familiar styled-components/Emotion API)
const Card = styled("div", { bg: "white", p: 4, _hover: { shadow: "lg" } });

// Global styles
globalCss({ body: { margin: 0 }, "*": { boxSizing: "border-box" } });

// Keyframes
const spin = keyframes({ from: { transform: "rotate(0)" }, to: { transform: "rotate(360deg)" } });
<Box animation={`${spin} 1s linear infinite`} />
```

## Theme (devup.json)

```json
{
  "theme": {
    "colors": {
      "default": { "primary": "#0070f3", "text": "#000" },
      "dark": { "primary": "#3291ff", "text": "#fff" }
    },
    "typography": {
      "heading": { "fontFamily": "Pretendard", "fontSize": "24px", "fontWeight": 700 }
    }
  }
}
```

Use with `$` prefix: `<Box color="$primary" typography="$heading" />`

Theme API:
```tsx
import { useTheme, setTheme, getTheme, initTheme, ThemeScript } from "@devup-ui/react";
setTheme("dark");        // switch theme
const theme = useTheme(); // hook for current theme
<ThemeScript />          // SSR hydration (add to <head>)
```

## Build Plugin Setup

```ts
// vite.config.ts
import DevupUI from "@devup-ui/vite-plugin";
export default defineConfig({ plugins: [react(), DevupUI()] });

// next.config.ts
import { DevupUI } from "@devup-ui/next-plugin";
export default DevupUI({
  // Next.js config here
});

// rsbuild.config.ts
import DevupUI from "@devup-ui/rsbuild-plugin";
export default defineConfig({ plugins: [DevupUI()] });
```

Options:
- `singleCss: true` - single CSS file (recommended for Turbopack)
- `include: ["@devup/hello"]` - process external libraries that use @devup-ui internally

```ts
// When using external library that uses @devup-ui (e.g. @devup/hello)
DevupUI({ include: ["@devup/hello"] })  // required to extract and merge their styles
```

## Anti-Patterns (NEVER do)

| Wrong | Right | Why |
|-------|-------|-----|
| `<Box style={{ color: "red" }}>` | `<Box color="red">` | style prop bypasses extraction |
| `css({ bg: variable })` | `<Box bg={variable}>` | css()/globalCss() only accept static values |
| No build plugin configured | Configure plugin first | Components throw at runtime without transformation |
| `as any` on style props | Fix types properly | Type errors indicate real issues |
