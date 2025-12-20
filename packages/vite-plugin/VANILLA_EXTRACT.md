# Vanilla Extract Support

DevUpUI now supports [Vanilla Extract](https://vanilla-extract.style/), a zero-runtime CSS-in-JS library that uses TypeScript.

## Installation

```bash
pnpm add -D @vanilla-extract/css @vanilla-extract/vite-plugin
```

## Usage

### Basic Setup

```ts
// vite.config.ts
import { DevupUI, withVanillaExtract } from '@devup-ui/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    react(),
    DevupUI({
      vanillaExtract: withVanillaExtract(),
    }),
  ],
})
```

### With Custom Options

```ts
// vite.config.ts
import { DevupUI, withVanillaExtract } from '@devup-ui/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    react(),
    DevupUI({
      vanillaExtract: withVanillaExtract({
        identifiers: 'short', // or 'debug' for readable class names
        emitCssInSsr: true, // emit CSS during SSR
      }),
    }),
  ],
})
```

### Using Your Own Vanilla Extract Plugin

```ts
// vite.config.ts
import { DevupUI } from '@devup-ui/vite-plugin'
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    react(),
    DevupUI({
      vanillaExtract: {
        plugin: vanillaExtractPlugin({
          identifiers: 'debug',
        }),
      },
    }),
  ],
})
```

### Simple Boolean Flag

```ts
// vite.config.ts
import { DevupUI } from '@devup-ui/vite-plugin'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    react(),
    DevupUI({
      vanillaExtract: true, // Uses default vanilla-extract settings
    }),
  ],
})
```

## Creating Styles

Create a `.css.ts` file:

```ts
// Button.css.ts
import { style } from '@vanilla-extract/css'

export const button = style({
  backgroundColor: '#007bff',
  color: 'white',
  border: 'none',
  padding: '0.75rem 1.5rem',
  borderRadius: '4px',
  fontSize: '1rem',
  cursor: 'pointer',
  transition: 'background-color 0.2s ease',
  ':hover': {
    backgroundColor: '#0056b3',
  },
})

export const primary = style({
  backgroundColor: '#28a745',
  ':hover': {
    backgroundColor: '#218838',
  },
})
```

Use the styles in your component:

```tsx
// Button.tsx
import * as styles from './Button.css'

export function Button({ variant = 'default', children }) {
  return (
    <button className={variant === 'primary' ? styles.primary : styles.button}>
      {children}
    </button>
  )
}
```

## Features

- ✅ **Type Safety**: Full TypeScript support with autocomplete
- ✅ **Zero Runtime**: All CSS is extracted at build time
- ✅ **Zero FOUC**: No flash of unstyled content
- ✅ **CSS Modules**: Scoped styles by default
- ✅ **Pseudo Selectors**: `:hover`, `:active`, `:focus`, etc.
- ✅ **Media Queries**: Responsive design support
- ✅ **Themes**: Built-in theming support

## API Reference

### `withVanillaExtract(options?)`

Helper function to create vanilla-extract configuration.

**Parameters:**

- `options` (optional): VanillaExtractOptions
  - `identifiers`: `'short' | 'debug'` - Class name format
  - `emitCssInSsr`: `boolean` - Emit CSS during SSR
  - `esbuildOptions`: `object` - Custom esbuild options

**Returns:** `VanillaExtractConfig`

### `createVanillaExtractConfig(options?)`

Alias for `withVanillaExtract()`.

## Example

See the complete example in `/apps/vite`:

- [VanillaExtractExample.css.ts](../../apps/vite/src/VanillaExtractExample.css.ts) - Style definitions
- [VanillaExtractExample.tsx](../../apps/vite/src/VanillaExtractExample.tsx) - Component using the styles
- [vite.config.ts](../../apps/vite/vite.config.ts) - Configuration

## Combining with DevUpUI Styles

You can use both DevUpUI's styled components and Vanilla Extract in the same project:

```tsx
import { Box } from '@devup-ui/react'

import * as styles from './MyComponent.css'

export function MyComponent() {
  return (
    <Box bg="$primary" p={4}>
      <div className={styles.card}>Mixed styling approach works great!</div>
    </Box>
  )
}
```

## Resources

- [Vanilla Extract Documentation](https://vanilla-extract.style/)
- [DevUpUI Documentation](https://devup-ui.com)
