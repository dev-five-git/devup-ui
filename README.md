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

<!-- benchmark:start -->
[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/37127187436) (commit `49b33534`, Devup UI 1.0.44) on `ubuntu-24.04` with Next.js 16.3.3. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`.

Build Size is every byte of the build output directory (`.next`, or `dist` for vinext); CSS Size is the emitted `.css` files alone, measured separately.

Webpack:

| Library                     | Version | Median Build Time | Range           | Build Size        | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | ----------------- | ------------ |
| tailwindcss                 | 4.3.3   | 12.57s            | 12.53s - 13.07s | 67,337,100 bytes  | 5,852 bytes  |
| styleX                      | 0.19.0  | 28.43s            | 28.39s - 28.90s | 96,290,977 bytes  | 425 bytes    |
| vanilla-extract             | 1.21.2  | 12.18s            | 12.10s - 12.49s | 68,577,913 bytes  | 294 bytes    |
| kuma-ui                     | 1.6.4   | 13.50s            | 13.23s - 14.56s | 75,627,639 bytes  | 340 bytes    |
| panda-css                   | 1.12.0  | 13.61s            | 13.44s - 14.35s | 71,844,396 bytes  | 15,615 bytes |
| chakra-ui                   | 3.37.0  | 20.19s            | 20.16s - 20.75s | 207,420,203 bytes | 0 bytes      |
| mui                         | 9.4.0   | 14.18s            | 14.03s - 14.44s | 101,478,873 bytes | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **10.97s**        | 10.83s - 11.25s | 67,438,054 bytes  | 562 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **10.98s**        | 10.90s - 11.03s | 67,415,063 bytes  | 790 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Build Size       | CSS Size    |
| -------------------------------------- | ------- | ----------------- | ------------- | ---------------- | ----------- |
| tailwindcss                            | 4.3.3   | 5.83s             | 5.55s - 5.90s | 38,422,653 bytes | 6,197 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **5.66s**         | 5.58s - 5.68s | 36,545,972 bytes | 327 bytes   |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **5.52s**         | 5.51s - 5.65s | 36,571,452 bytes | 268 bytes   |

The Turbopack ranges overlap, so the direct-API median is 0.17s (2.9%) ahead of Tailwind on this fixture and the two are effectively at parity. The 3 cold samples were Tailwind `5.90s, 5.55s, 5.83s`, direct Devup UI `5.66s, 5.58s, 5.68s`, and static `.css.ts` `5.65s, 5.52s, 5.51s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
<!-- benchmark:end -->

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
const boxExample = <Box typography="heading" />
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
