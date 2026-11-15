<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor
</h3>

---

<div>
<img src='https://img.shields.io/npm/v/@devup-ui/react'>
<img src='https://img.shields.io/bundlephobia/minzip/@devup-ui/react'>
<img alt="Github Checks" src="https://badgen.net/github/checks/dev-five-git/devup-ui"/>
<img alt="Apache-2.0 License" src="https://img.shields.io/github/license/dev-five-git/devup-ui"/>
<a href="https://www.npmjs.com/package/@devup-ui/react">
<img alt="NPM Downloads" src="https://img.shields.io/npm/dm/@devup-ui/react.svg?style=flat"/>
</a>
<a href="https://badgen.net/github/stars/dev-five-git/devup-ui">
<img alt="Github Stars" src="https://badgen.net/github/stars/dev-five-git/devup-ui" />
</a>
<a href="https://discord.gg/8zjcGc7cWh">
<img alt="Discord" src="https://img.shields.io/discord/1321362173619994644.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2" />
</a>
<a href="https://codecov.io/gh/dev-five-git/devup-ui" >
 <img src="https://codecov.io/gh/dev-five-git/devup-ui/graph/badge.svg?token=8I5GMB2X5B"/>
</a>
</div>

---

English | [한국어](https://github.com/dev-five-git/devup-ui/blob/main/README_ko.md)

## Install

```sh
npm install @devup-ui/react

# on next.js
npm install @devup-ui/next-plugin

# on vite
npm install @devup-ui/vite-plugin

# on rsbuild
npm install @devup-ui/rsbuild-plugin

# on webpack
npm install @devup-ui/webpack-plugin
```

## Features

- **Preprocessor** — All CSS extraction happens at build time
- **Zero Config** — Works out of the box with sensible defaults
- **Zero FOUC** — No flash of unstyled content, no Provider required
- **Zero Runtime** — No client-side JavaScript for styling
- **RSC Support** — Full React Server Components compatibility
- **Library Mode** — Build component libraries with extracted styles
- **Dynamic Themes** — Zero-cost theme switching via CSS variables
- **Type-Safe Themes** — Full TypeScript support for theme tokens
- **Smallest & Fastest** — Proven by benchmarks

## Inspirations

- Styled System
- Chakra UI
- Theme UI
- Vanilla Extract
- Rainbow Sprinkles
- Kuma UI

## Comparison Benchmarks

The benchmark tables live in the [repository README](https://github.com/dev-five-git/devup-ui#comparison-benchmarks), which CI regenerates from each checked benchmark run, so this package README does not copy them.

## How it works

Devup UI transforms your components at build time. Class names are generated using a compact base-37 encoding for minimal CSS size. What the build cannot know is kept as a CSS variable or reported as a [located build error](https://devup-ui.com/docs/build-errors); see [supported syntax & limitations](https://devup-ui.com/docs/limitations).

**Basic transformation:**

Generated class and CSS variable names may change between versions, build state, and prefix settings; do not manually reuse them.

You write:

```tsx
import { Box } from '@devup-ui/react'

const example = <Box _hover={{ bg: 'blue' }} bg="red" p={4} />
```

Devup UI generates:

```tsx
import "@devup-ui/react/devup-ui.css";
const example = <div className="a b c" />;
```

With CSS:

```css
/*! devup-ui v1.0.82, | Apache License 2.0 | https://devup-ui.com */.b{background:red}.c{padding:16px}.a:hover{background:blue}
```

**Dynamic values become CSS variables:**

You write:

```tsx
import { Box } from '@devup-ui/react'

const example = <Box bg={colorVariable} />
```

Devup UI generates:

```tsx
import "@devup-ui/react/devup-ui.css";
const example = <div className="a" style={{ "--b": colorVariable }} />;
```

With CSS:

```css
/*! devup-ui v1.0.82, | Apache License 2.0 | https://devup-ui.com */.a{background:var(--b)}
```

**Complex expressions and responsive arrays:**

You write:

```tsx
import { Box } from '@devup-ui/react'

const example = <Box bg={['red', 'blue', isActive ? 'green' : dynamicColor]} />
```

Devup UI generates:

```tsx
import "@devup-ui/react/devup-ui.css";
const example = <div className={`a b ${isActive ? "c" : "d"}`} style={{ "--e": dynamicColor }} />;
```

With responsive CSS for each breakpoint:

```css
/*! devup-ui v1.0.82, | Apache License 2.0 | https://devup-ui.com */.a{background:red}@media(min-width:480px){.b{background:blue}}@media(min-width:768px){.c{background:green}.d{background:var(--e)}}
```

**Type-safe theming:**

`devup.json`

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
      "heading": {
        "fontFamily": "Pretendard",
        "fontSize": "24px",
        "fontWeight": 700,
        "lineHeight": 1.3
      }
    }
  }
}
```

```tsx
// Type-safe theme tokens
const textExample = <Text color="$primary" />
const boxExample = <Box typography="heading" />
```

**Responsive + Pseudo selectors together:**

```tsx
// Responsive with pseudo selector
const example = <Box _hover={{ bg: ['red', 'blue'] }} />

// Equivalent syntax
const example2 = <Box _hover={[{ bg: 'red' }, { bg: 'blue' }]} />
```

**styled-components / Emotion compatible `styled()` API:**

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
  px: 4, // 4 * 4 = 16px
  py: 2, // 2 * 4 = 8px
  borderRadius: '4px',
  cursor: 'pointer',
})

// Usage
const cardExample = <Card>Content</Card>
const buttonExample = <Button>Click me</Button>
```

## Custom Shorthands

Custom shorthands belong to the plugin options rather than `devup.json` because
they extend build-time extraction, not design tokens.

```ts
// next.config.ts
const nextConfig = {}

export default DevupUI(nextConfig, {
  shorthands: {
    insetX: ['left', 'right'],
    scrollMarginX: ['scrollMarginLeft', 'scrollMarginRight'],
  },
})
```

Every target receives the same value, and target names accept camelCase or
kebab-case. The generated `df/theme.d.ts` adds type completion for direct,
responsive, pseudo-selector, and custom-selector usage. Restart Next.js after
changing the option.
If your `tsconfig.json` does not already match `df/*.d.ts`, add it (or your
custom `distDir`) to `include`.

```tsx
<Box insetX={[0, null, 'auto']} _hover={{ insetX: 4 }} />
```

## Extraction engine

The build-time compiler defaults to `native` for Webpack and Turbopack. Select
WASM explicitly with `DevupUI(nextConfig, { engine: 'wasm' })`. The chosen engine
is retained by configuration, prewarm, extraction, CSS and delayed callbacks.
Turbopack loader options carry only the engine string; the coordinator retains
the selected physical compiler. Independent concurrent builds in one process
are unsupported.

Native load failures identify the required platform package and retain the
original cause. There is no automatic fallback to WASM. Install optional native
dependencies normally, or choose WASM explicitly for an unsupported platform.
Both engines extract CSS during the build and add no client styling runtime.

## Turbopack build profiling

Set `DEVUP_UI_PROFILE=1` for an opt-in, structured timing log during a
Turbopack build.

```bash
DEVUP_UI_PROFILE=1 bun run build
```

```powershell
$env:DEVUP_UI_PROFILE = '1'; bun run build
```

Each `[devup-ui:profile]` JSON entry reports one phase. `next.graph` measures
the static import-graph pre-pass, `next.prewarm` measures production extraction
before Turbopack starts loaders, and `coordinator.extract` separates request,
compiler extraction, CSS/state serialization, and write time for each loaded
module. The setting is disabled by default and does not collect timings or
write logs when it is absent.
