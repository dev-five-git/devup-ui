---
name: devup-ui
description: A zero-runtime CSS-in-JS preprocessor framework for React. Use this skill when working with Devup UI components, styling, theming, or build configuration. This skill covers component usage (Box, Flex, Grid, Text, Button, etc.), styling APIs (css, styled, globalCss, keyframes), theme configuration, and build plugin setup for Vite, Next.js, Webpack, and Rsbuild.
---

# Devup UI

Devup UI is a zero-runtime CSS-in-JS preprocessor that transforms styles at build time using Rust and WebAssembly. All styling is processed during compilation, resulting in zero runtime overhead, zero FOUC, and dynamic theme support via CSS variables.

## Project Structure

```
devup-ui/
├── packages/
│   ├── react/           # Core React library (@devup-ui/react)
│   ├── components/      # Pre-built UI components (@devup-ui/components)
│   ├── vite-plugin/     # Vite build plugin
│   ├── next-plugin/     # Next.js integration
│   ├── webpack-plugin/  # Webpack build plugin
│   ├── rsbuild-plugin/  # Rsbuild integration
│   ├── reset-css/       # CSS reset utility
│   └── eslint-plugin/   # Custom ESLint rules
├── libs/                # Rust libraries (core processing)
│   ├── extractor/       # JSX/TSX AST parser and CSS extraction
│   ├── sheet/           # CSS sheet generation
│   └── css/             # CSS utilities
├── bindings/
│   └── devup-ui-wasm/   # WebAssembly bindings
├── apps/                # Example applications
└── benchmark/           # Performance benchmarks
```

## Core Concepts

### Build-Time Transformation

Components are transformed at build time. Class names are generated using a compact base-37 encoding (a-z, 0-9, \_) for minimal CSS size:

```tsx
// Developer writes:
const example = <Box _hover={{ bg: 'blue' }} bg="red" p={4} />

// Transformed to:
const generated = <div className="a b c" />

// Generated CSS:
// .a { background-color: red; }
// .b { padding: 1rem; }
// .c:hover { background-color: blue; }
```

Class name sequence: `a`, `b`, ... `z`, `_`, `aa`, `ab`, ... `az`, `a0`, ... `a9`, `a_`, `ba`, ...

Dynamic values become CSS variables:

```tsx
// Developer writes:
const example = <Box bg={colorVariable} />

// Transformed to:
const generated = <div className="a" style={{ '--a': colorVariable }} />

// Generated CSS:
// .a { background-color: var(--a); }
```

## Components

All components are from `@devup-ui/react`:

- `Box` - Base layout component with style props
- `Flex` - Flexbox container
- `Grid` - CSS Grid container
- `VStack` / `HStack` - Vertical/horizontal stacking
- `Center` - Center content
- `Text` - Typography component
- `Button` - Interactive button
- `Input` - Form input
- `Image` - Image component

### Style Props

Components accept style props directly:

```tsx
<Box
  _dark={{ bg: 'gray' }} // Theme variant
  _hover={{ bg: 'blue' }} // Pseudo-selector
  bg="red"
  color="white"
  m={[2, 4]} // Responsive: mobile=2, desktop=4
  p={4}
/>
```

## Styling APIs

### css()

Create reusable style objects:

```tsx
import { css } from '@devup-ui/react'

const styles = css({
  bg: 'red',
  p: 4,
  _hover: { bg: 'blue' },
})

const example = <Box {...styles} />
```

### styled()

Create styled components (compatible with styled-components and Emotion patterns):

```tsx
import { styled } from '@devup-ui/react'

// Familiar syntax for styled-components and Emotion users
const Card = styled('div', {
  bg: 'white',
  p: 4, // 4 * 4 = 16px
  borderRadius: '8px',
  boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)',
  _hover: {
    boxShadow: '0 10px 15px rgba(0, 0, 0, 0.1)',
  },
})

const Button = styled('button', {
  bg: 'blue',
  color: 'white',
  px: 4, // 4 * 4 = 16px
  py: 2, // 2 * 4 = 8px
  borderRadius: '4px',
  cursor: 'pointer',
  _hover: { bg: 'darkblue' },
  _active: { bg: 'navy' },
})
```

### globalCss()

Define global styles:

```tsx
import { globalCss } from '@devup-ui/react'

globalCss({
  body: { margin: 0 },
  '*': { boxSizing: 'border-box' },
})
```

### keyframes()

Define CSS animations:

```tsx
import { keyframes } from '@devup-ui/react'

const spin = keyframes({
  from: { transform: 'rotate(0deg)' },
  to: { transform: 'rotate(360deg)' },
})

const example = <Box animation={`${spin} 1s linear infinite`} />
```

## Theme Configuration

Create `devup.json` in project root:

```json
{
  "theme": {
    "colors": {
      "default": {
        "primary": "#0070f3",
        "text": "#000"
      },
      "dark": {
        "primary": "#3291ff",
        "text": "#fff"
      }
    },
    "typography": {
      "bold": {
        "fontFamily": "Pretendard",
        "fontSize": "14px",
        "fontWeight": 800,
        "lineHeight": 1.3,
        "letterSpacing": "-0.03em"
      },
      "h1": [
        {
          "fontFamily": "Pretendard",
          "fontSize": "38px",
          "fontWeight": 800,
          "lineHeight": 1.3
        },
        null,
        null,
        null,
        {
          "fontFamily": "Pretendard",
          "fontSize": "52px",
          "fontWeight": 800,
          "lineHeight": 1.3
        }
      ]
    }
  }
}
```

### Theme API

```tsx
import {
  getTheme,
  initTheme,
  setTheme,
  ThemeScript,
  useTheme,
} from '@devup-ui/react'

// Get current theme (inside component)
function MyComponent() {
  const theme = useTheme()
  return null
}

// Set theme
setTheme('dark')

// Initialize theme (SSR)
initTheme()

// Get theme value
const currentTheme = getTheme()

// Hydration script (add to HTML head)
const themeScript = <ThemeScript />
```

## Build Plugin Configuration

### Vite

```ts
// vite.config.ts
import DevupUI from '@devup-ui/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    react(),
    DevupUI({
      package: '@devup-ui/react', // Target package
      cssDir: 'df/devup-ui', // CSS output directory
      devupFile: 'devup.json', // Theme config file
      extractCss: true, // Enable CSS extraction
      singleCss: false, // Single vs per-file CSS
      debug: false, // Debug mode
      include: [], // Additional packages to process
    }),
  ],
})
```

### Next.js

```js
// next.config.js
const withDevupUI = require('@devup-ui/next-plugin')

module.exports = withDevupUI({
  // Next.js config
})
```

### Webpack

```js
// webpack.config.js
const DevupUIPlugin = require('@devup-ui/webpack-plugin')

module.exports = {
  plugins: [new DevupUIPlugin()],
}
```

### Rsbuild

```ts
// rsbuild.config.ts
import DevupUI from '@devup-ui/rsbuild-plugin'
import { defineConfig } from '@rsbuild/core'

export default defineConfig({
  plugins: [DevupUI()],
})
```

## Development Commands

```bash
# Install dependencies
bun install

# Build all packages
bun run build

# Run development servers
bun run dev

# Run tests
bun run test

# Run linting
bun run lint

# Run benchmarks
bun run benchmark
```

## Guidelines

- All Devup UI components throw errors at runtime - they must be transformed by the build plugin
- Use responsive arrays for breakpoint-based styles: `p={[2, 4, 6]}`
- Use underscore prefix for pseudo-selectors: `_hover`, `_focus`, `_active`, `_dark`
- Theme values are accessed via CSS variables at runtime for zero-cost theme switching
- Generated CSS is output to `df/devup-ui/` directory by default
- TypeScript theme types are generated at `df/theme.d.ts`

## Examples

### Basic Component Usage

```tsx
import { Box, Button, Flex, Text } from '@devup-ui/react'

function Card() {
  return (
    <Box bg="white" borderRadius="lg" boxShadow="md" p={4}>
      <Text fontSize="lg" fontWeight="bold">
        Title
      </Text>
      <Text color="gray.600">Description</Text>
      <Flex gap={2} mt={4}>
        <Button bg="primary" color="white">
          Action
        </Button>
      </Flex>
    </Box>
  )
}
```

### Theme-Aware Component

```tsx
import { Box } from '@devup-ui/react'

function ThemeCard() {
  return (
    <Box
      _dark={{ bg: '$background', color: '$text' }}
      bg="$background"
      color="$text"
      p={4}
    >
      This adapts to the current theme
    </Box>
  )
}
```

### Dynamic Styling

```tsx
import { Box } from '@devup-ui/react'

function DynamicBox({ isActive, color }) {
  return (
    <Box
      bg={isActive ? 'blue' : 'gray'}
      color={color} // Dynamic value becomes CSS variable
      p={4}
    />
  )
}
```
