<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    The Future of CSS-in-JS — Zero Runtime, Full Power
</h3>

<p align="center">
    <strong>Zero Config · Zero FOUC · Zero Runtime · Complete CSS-in-JS Syntax Coverage</strong>
</p>

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

English | [한국어](README_ko.md)

## Why Devup UI?

**Devup UI isn't just another CSS-in-JS library — it's the next evolution.**

Traditional CSS-in-JS solutions force you to choose between developer experience and performance. Devup UI eliminates this trade-off entirely by processing all styles at build time using a Rust-powered preprocessor.

- **Complete Syntax Coverage**: Every CSS-in-JS pattern you know — variables, conditionals, responsive arrays, pseudo-selectors — all fully supported
- **Familiar API**: `styled()` API compatible with styled-components and Emotion patterns
- **True Zero Runtime**: No JavaScript execution for styling at runtime. Period.
- **Smallest Bundle Size**: Optimized class names (`a`, `b`, ... `aa`, `ab`) minimize CSS output
- **Fastest Build Times**: Rust + WebAssembly delivers unmatched preprocessing speed

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

## Comparison Benchmarks

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/33254030962) on `ubuntu-latest` with Next.js 16.3.3. All Next.js builds use the native TypeScript 7 CLI for type checking.

Webpack values are one cold build:

| Library                     | Version | Build Time | Build Size        |
| --------------------------- | ------- | ---------- | ----------------- |
| tailwindcss                 | 4.3.3   | 12.81s     | 66,453,190 bytes  |
| styleX                      | 0.19.0  | 27.61s     | 95,413,403 bytes  |
| vanilla-extract             | 1.21.2  | 11.96s     | 67,689,923 bytes  |
| kuma-ui                     | 1.6.4   | 13.07s     | 74,747,637 bytes  |
| panda-css                   | 1.12.0  | 13.20s     | 70,970,208 bytes  |
| chakra-ui                   | 3.37.0  | 19.98s     | 206,589,898 bytes |
| mui                         | 9.4.0   | 13.78s     | 100,603,186 bytes |
| **devup-ui (per-file CSS)** | 1.0.40  | **10.73s** | 66,535,095 bytes  |
| **devup-ui (single CSS)**   | 1.0.40  | **10.81s** | 66,535,321 bytes  |

Turbopack values are medians of six cold builds in alternating order:

| Library                                | Version | Median Build Time | Build Size           |
| -------------------------------------- | ------- | ----------------- | -------------------- |
| tailwindcss                            | 4.3.3   | 5.52s             | 38,425,132 bytes     |
| **devup-ui (direct APIs, single CSS)** | 1.0.40  | **5.46s**         | **36,507,782 bytes** |
| **devup-ui (static `.css.ts`)**        | 1.0.40  | **5.41s**         | 36,586,883 bytes     |

The Turbopack ranges overlap, so the direct-API median is 0.06s (1.1%) ahead but still effectively parity with Tailwind on this fixture. The six cold samples were Tailwind `5.42, 5.56, 5.70, 5.53, 5.41, 5.52s`, direct Devup UI `5.64, 5.40, 5.42, 5.46, 5.54, 5.46s`, and static `.css.ts` `5.42, 5.38, 5.42, 5.47, 5.40, 5.38s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. The static `.css.ts` row uses the `lite` WASM fast path; dynamic `.css.ts` modules use the full Boa evaluator and are not represented by that median.

## How it works

Devup UI transforms your components at build time. Class names are generated using a compact base-37 encoding for minimal CSS size.

**Basic transformation:**

```tsx
// You write:
const variable = <Box _hover={{ bg: 'blue' }} bg="red" p={4} />

// Devup UI generates:
const variable = <div className="a b c" />

// With CSS:
// .a { background-color: red; }
// .b { padding: 1rem; }
// .c:hover { background-color: blue; }
```

**Dynamic values become CSS variables:**

```tsx
// You write:
const example = <Box bg={colorVariable} />

// Devup UI generates:
const example = <div className="a" style={{ '--a': colorVariable }} />

// With CSS:
// .a { background-color: var(--a); }
```

**Complex expressions and responsive arrays — fully supported:**

```tsx
// You write:
const example = <Box bg={['red', 'blue', isActive ? 'green' : dynamicColor]} />

// Devup UI generates:
const example = (
  <div
    className={`a b ${isActive ? 'c' : 'd'}`}
    style={{ '--d': dynamicColor }}
  />
)

// With responsive CSS for each breakpoint
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
const boxExample = <Box typography="$heading" />
```

**Custom shorthands:**

Define reusable prop aliases in the build plugin options. Property names may
use camelCase or CSS kebab-case, and one shorthand can target multiple
properties.

```ts
// vite.config.ts
export default defineConfig({
  plugins: [
    DevupUI({
      shorthands: {
        insetX: ['left', 'right'],
        scrollMarginX: ['scrollMarginLeft', 'scrollMarginRight'],
      },
    }),
  ],
})
```

The plugin adds configured names to the generated declaration file, so they are
available as type-safe component props and selector properties after the plugin
has run. Restart the build after changing this option so `df/theme.d.ts` is
regenerated. The same value is applied to every target property.

If your `tsconfig.json` limits `include` to `src`, also include `df/*.d.ts` (or
the matching custom `distDir`) so the generated augmentation is loaded.

```tsx
const pinned = <Box insetX={0} />
const responsive = <Box insetX={[0, null, 'auto']} />
const interactive = <Box _hover={{ insetX: 4 }} />
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

## Inspirations

- Styled System
- Chakra UI
- Theme UI
- Vanilla Extract
- Rainbow Sprinkles
- Kuma UI

## How to Contribute

### Requirements

- [Node.js](https://nodejs.org) (LTS version recommended)
- [Rust](https://rustup.rs) compiler
- [Bun](https://bun.sh) package manager

### Development Setup

To set up the development environment, install the following packages:

```sh
bun install
bun run build
cargo install cargo-tarpaulin

cargo install wasm-pack
```

After installation, run `bun run test` to ensure everything works correctly.
