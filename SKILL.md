---
name: devup-ui
description: |
  Zero-runtime CSS-in-JS preprocessor for React. Transforms JSX styles to static CSS at build time.

  TRIGGER WHEN:
  - Writing/modifying Devup UI components (Box, Flex, Grid, Text, Button, etc.)
  - Using styling APIs: css(), globalCss(), keyframes()
  - Configuring devup.json theme (colors, typography, extends)
  - Setting up build plugins (Vite, Next.js, Webpack, Rsbuild, Bun)
  - Debugging "Cannot run on the runtime" errors
  - Working with responsive arrays, pseudo-selectors (_hover, _dark, etc.)
  - Using polymorphic `as` prop or `selectors` prop
  - Working with @devup-ui/components (Button, Input, Select, Toggle, etc.)
---

# Devup UI

Build-time CSS extraction. No runtime JS for styling.

## Critical: Components Are Compile-Time Only

All `@devup-ui/react` components throw `Error('Cannot run on the runtime')`. They are **placeholders** that build plugins transform to native HTML elements with classNames.

```tsx
// BEFORE BUILD (what you write):
<Box bg="red" p={4} _hover={{ bg: "blue" }} />

// AFTER BUILD (what runs in browser):
<div className="a b c" />  // + CSS: .a{background:red} .b{padding:16px} .c:hover{background:blue}
```

## Components

### @devup-ui/react (Layout Primitives)

All are polymorphic (accept `as` prop). Default element is `<div>` unless noted.

| Component | Default Element | Purpose |
|-----------|----------------|---------|
| `Box` | `div` | Base layout primitive, accepts all style props |
| `Flex` | `div` | Flexbox container (shorthand for `display: flex`) |
| `Grid` | `div` | CSS Grid container |
| `VStack` | `div` | Vertical stack (flex column) |
| `Center` | `div` | Centered content |
| `Text` | `p` | Text/typography |
| `Image` | `img` | Image element |
| `Input` | `input` | Input element |
| `Button` | `button` | Button element |
| `ThemeScript` | -- | SSR theme hydration (add to `<head>`) |

### @devup-ui/components (Pre-built UI)

Higher-level components with built-in behavior. These are **runtime components** (not compile-time only).

| Component | Key Props |
|-----------|-----------|
| `Button` | `variant` (`primary`/`default`), `size` (`sm`/`md`/`lg`), `loading`, `danger`, `icon`, `colors` |
| `Checkbox` | `children` (label), `onChange(checked)`, `colors` |
| `Input` | `error`, `errorMessage`, `allowClear`, `icon`, `typography`, `colors` |
| `Textarea` | `error`, `errorMessage`, `typography`, `colors` |
| `Radio` | `variant` (`default`/`button`), `colors` |
| `RadioGroup` | `options[]`, `direction` (`row`/`column`), `variant`, `value`, `onChange` |
| `Toggle` | `variant` (`default`/`switch`), `value`, `onChange(boolean)`, `colors` |
| `Select` | `type` (`default`/`radio`/`checkbox`), `options[]`, `value`, `onChange`, `colors` |
| `Stepper` | `min`, `max`, `type` (`input`/`text`), `value`, `onValueChange` |

**Select compound:** `SelectTrigger`, `SelectContainer`, `SelectOption`, `SelectDivider`
**Stepper compound:** `StepperContainer`, `StepperDecreaseButton`, `StepperIncreaseButton`, `StepperInput`
**Hooks:** `useSelect()`, `useStepper()`

All components accept a `colors` prop object for runtime color customization via CSS variables.

## Style Prop Syntax

### Shorthand Props (ALWAYS prefer these)

**Spacing (unitless number x 4 = px)**

| Shorthand | CSS Property |
|-----------|-------------|
| `m`, `mt`, `mr`, `mb`, `ml`, `mx`, `my` | margin-* |
| `p`, `pt`, `pr`, `pb`, `pl`, `px`, `py` | padding-* |

**Sizing**

| Shorthand | CSS Property |
|-----------|-------------|
| `w` | width |
| `h` | height |
| `minW`, `maxW` | min-width, max-width |
| `minH`, `maxH` | min-height, max-height |
| `boxSize` | width + height (same value) |

**Background**

| Shorthand | CSS Property |
|-----------|-------------|
| `bg` | background |
| `bgColor` | background-color |
| `bgImage`, `bgImg`, `backgroundImg` | background-image |
| `bgSize` | background-size |
| `bgPosition`, `bgPos` | background-position |
| `bgPositionX`, `bgPosX` | background-position-x |
| `bgPositionY`, `bgPosY` | background-position-y |
| `bgRepeat` | background-repeat |
| `bgAttachment` | background-attachment |
| `bgClip` | background-clip |
| `bgOrigin` | background-origin |
| `bgBlendMode` | background-blend-mode |

**Border**

| Shorthand | CSS Property |
|-----------|-------------|
| `borderTopRadius` | border-top-left-radius + border-top-right-radius |
| `borderBottomRadius` | border-bottom-left-radius + border-bottom-right-radius |
| `borderLeftRadius` | border-top-left-radius + border-bottom-left-radius |
| `borderRightRadius` | border-top-right-radius + border-bottom-right-radius |

**Layout & Position**

| Shorthand | CSS Property |
|-----------|-------------|
| `flexDir` | flex-direction |
| `pos` | position |
| `positioning` | Helper: `"top"`, `"bottom-right"`, etc. (sets edges to 0) |
| `objectPos` | object-position |
| `offsetPos` | offset-position |
| `maskPos` | mask-position |
| `maskImg` | mask-image |

**Typography**

| Shorthand | Effect |
|-----------|--------|
| `typography` | Applies theme typography token (fontFamily, fontSize, fontWeight, lineHeight, letterSpacing) |

All standard CSS properties from `csstype` are also accepted directly (e.g., `display`, `gap`, `opacity`, `transform`, `animation`, etc.).

### Spacing Scale (unitless number x 4 = px)

```tsx
<Box p={1} />    // padding: 4px
<Box p={4} />    // padding: 16px
<Box p="4" />    // padding: 16px (unitless string also x 4)
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
  _focusVisible={{ outlineColor: "$primary" }}
  _active={{ bg: "darkblue" }}
  _disabled={{ opacity: 0.5 }}
  _before={{ content: '""' }}
  _after={{ content: '""' }}
  _firstChild={{ mt: 0 }}
  _lastChild={{ mb: 0 }}
  _placeholder={{ color: "gray" }}
/>
```

All CSS pseudo-classes and pseudo-elements from `csstype` are supported with `_camelCase` naming.

### Group Selectors

Style children based on parent state:

```tsx
<Box _groupHover={{ color: "blue" }} />
<Box _groupFocus={{ outline: "2px solid" }} />
<Box _groupActive={{ bg: "darkblue" }} />
```

### Theme Selectors

```tsx
<Box _themeDark={{ bg: "gray.900" }} />
<Box _themeLight={{ bg: "white" }} />
```

### At-Rules (Media, Container, Supports)

```tsx
// Underscore prefix syntax
<Box _print={{ display: "none" }} />
<Box _screen={{ display: "block" }} />
<Box _media={{ "(min-width: 768px)": { w: "50%" } }} />
<Box _container={{ "(min-width: 400px)": { p: 4 } }} />
<Box _supports={{ "(display: grid)": { display: "grid" } }} />

// @ prefix syntax (equivalent)
<Box {...{ "@media": { "(min-width: 768px)": { w: "50%" } } }} />
```

### Custom Selectors

```tsx
<Box selectors={{
  "&:hover": { color: "red" },
  "&::before": { content: '">"' },
  "&:nth-child(2n)": { bg: "gray" },
}} />
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

### Responsive + Pseudo Combined

```tsx
<Box _hover={{ bg: ['red', 'blue'] }} />
// Alternative syntax:
<Box _hover={[{ bg: 'red' }, { bg: 'blue' }]} />
```

## Special Props

### `as` (Polymorphic Element)

Changes the rendered HTML element or renders a custom component:

```tsx
<Box as="section" bg="gray" />         // renders <section>
<Box as="a" href="/about" />           // renders <a>
<Box as={MyComponent} bg="red" />      // renders <MyComponent> with extracted styles
<Box as={b ? "div" : "section"} />     // conditional element type
```

### `props` (Pass-Through to `as` Component)

When `as` is a custom component, use `props` to pass component-specific props:

```tsx
<Box as={MotionDiv} w="100%" props={{ animate: { duration: 1 } }} />
```

### `styleVars` (Manual CSS Variable Injection)

```tsx
<Box styleVars={{ "--custom-color": dynamicValue }} bg="var(--custom-color)" />
```

### `styleOrder` (CSS Cascade Priority)

Controls specificity when combining `className` with direct props. **Required** when mixing `css()` classNames with inline style props.

```tsx
<Box className={cardStyle} bg="$background" styleOrder={1} />
// Conditional styleOrder
<Box bg="red" styleOrder={isActive ? 1 : 0} />
```

## Styling APIs

### css() Returns className String (NOT object)

```tsx
import { css, globalCss, keyframes } from "@devup-ui/react";
import clsx from "clsx";

// css() returns a className STRING
const cardStyle = css({ bg: "white", p: 4, borderRadius: "8px" });
<div className={cardStyle} />

// Combine with clsx
const baseStyle = css({ p: 4, borderRadius: "8px" });
const activeStyle = css({ bg: "$primary", color: "white" });
<Box className={clsx(baseStyle, isActive && activeStyle)} styleOrder={1} />
```

### globalCss() and keyframes()

```tsx
globalCss({ body: { margin: 0 }, "*": { boxSizing: "border-box" } });

const spin = keyframes({ from: { transform: "rotate(0)" }, to: { transform: "rotate(360deg)" } });
<Box animation={`${spin} 1s linear infinite`} />
```

### Dynamic Values with Custom Components

`css()` only accepts **static values**. For dynamic values on custom components, use `<Box as={Component}>`:

```tsx
// WRONG - css() cannot handle dynamic values
<CustomComponent className={css({ w: width })} />

// CORRECT - Box with as prop handles dynamic values via CSS variables
<Box as={CustomComponent} w={width} />
```

## Theme (devup.json)

```json
{
  "extends": ["./base-theme.json"],
  "theme": {
    "colors": {
      "default": { "primary": "#0070f3", "text": "#000", "bg": "#fff" },
      "dark": { "primary": "#3291ff", "text": "#fff", "bg": "#111" }
    },
    "typography": {
      "heading": {
        "fontFamily": "Pretendard",
        "fontSize": "24px",
        "fontWeight": 700,
        "lineHeight": 1.3,
        "letterSpacing": "-0.02em"
      },
      "body": [
        { "fontSize": "14px", "lineHeight": 1.5 },
        null,
        { "fontSize": "16px", "lineHeight": 1.6 }
      ]
    }
  }
}
```

- **Colors**: Use with `$` prefix in JSX props: `<Box color="$primary" />`
- **Typography**: Use with `$` prefix: `<Text typography="$heading" />`
- **extends**: Inherit from base config files (deep merge, last wins)
- **Responsive typography**: Use arrays with `null` for unchanged breakpoints

Theme types are auto-generated via module augmentation of `DevupTheme` and `DevupThemeTypography`.

### Theme API

```tsx
import { useTheme, setTheme, getTheme, initTheme, ThemeScript } from "@devup-ui/react";

setTheme("dark");             // Switch theme (sets data-theme + localStorage)
const theme = getTheme();     // Get current theme name
const theme = useTheme();     // React hook (reactive)
initTheme();                  // Initialize on startup (auto-detect system preference)
<ThemeScript />               // SSR hydration script (add to <head>, prevents FOUC)
```

## Build Plugin Setup

### Vite

```ts
import DevupUI from "@devup-ui/vite-plugin";
export default defineConfig({ plugins: [react(), DevupUI()] });
```

### Next.js

```ts
import { DevupUI } from "@devup-ui/next-plugin";
export default DevupUI({ /* Next.js config */ });
```

### Rsbuild

```ts
import DevupUI from "@devup-ui/rsbuild-plugin";
export default defineConfig({ plugins: [DevupUI()] });
```

### Webpack

```ts
import { DevupUIWebpackPlugin } from "@devup-ui/webpack-plugin";
// Add to plugins array
```

### Bun

```ts
import { plugin } from "@devup-ui/bun-plugin";
// Auto-registers, always uses singleCss: true
```

### Plugin Options

```ts
DevupUI({
  singleCss: true,           // Single CSS file (recommended for Turbopack)
  include: ["@devup/hello"],  // Process external libs using @devup-ui
  prefix: "du",              // Class name prefix (e.g., "du-a" instead of "a")
  debug: true,               // Enable debug logging
  importAliases: {           // Redirect imports from other CSS-in-JS libs
    "@emotion/styled": "styled",        // default: enabled
    "styled-components": "styled",      // default: enabled
    "@vanilla-extract/css": true,       // default: enabled
  },
})
```

## $color Token Scope

`$color` tokens only work in **JSX props**. Use `var(--color)` in external objects.

```tsx
// CORRECT - $color in JSX prop
<Box bg="$primary" />
<Box bg={{ active: '$primary', inactive: '$gray' }[status]} />

// WRONG - $color in external object (won't be transformed)
const colors = { active: '$primary' }
<Box bg={colors.active} />  // broken!

// CORRECT - var(--color) in external object
const colors = { active: 'var(--primary)' }
<Box bg={colors.active} />
```

## Inline Variant Pattern (Preferred)

Use inline object indexing instead of external config objects:

```tsx
// PREFERRED - inline object indexing (build-time extractable)
<Box
  h={{ lg: '48px', md: '40px', sm: '32px' }[size]}
  bg={{ primary: '$primary', secondary: '$gray100' }[variant]}
/>

// AVOID - external config object (becomes dynamic, uses CSS variables)
const sizeStyles = { lg: { h: '48px' }, md: { h: '40px' } }
<Box h={sizeStyles[size].h} />
```

## Anti-Patterns (NEVER do)

| Wrong | Right | Why |
|-------|-------|-----|
| `<Box style={{ color: "red" }}>` | `<Box color="red">` | style prop bypasses extraction |
| `<Box {...css({...})} />` | `<Box className={css({...})} />` | css() returns string, not object |
| `css({ bg: variable })` | `<Box bg={variable}>` or `<Box as={Comp} bg={variable}>` | css()/globalCss() only accept static values |
| `$color` in external object | `var(--color)` in external object | $color only transformed in JSX props |
| No build plugin configured | Configure plugin first | Components throw at runtime without transformation |
| `as any` on style props | Fix types properly | Type errors indicate real issues |
| `@ts-ignore` / `@ts-expect-error` | Fix the type issue | Suppression hides real problems |
| `background="red"` | `bg="red"` | Always use shorthands |
| `padding={4}` | `p={4}` | Always use shorthands |
| `width="100%"` | `w="100%"` | Always use shorthands |
| `styled("div", {...})` | `<Box bg="red" />` | Use Box component with props, not styled() |
| `stylex.create({...})` | `<Box bg="red" />` | Use Box component with props, not stylex |
