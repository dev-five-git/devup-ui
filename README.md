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

Next.js Build Time and Build Size (github action - ubuntu-latest)

| Library                            | Version | Build Time | Build Size           |
| ---------------------------------- | ------- | ---------- | -------------------- |
| tailwindcss                        | 4.1.13  | 19.31s     | 59,521,539 bytes     |
| styleX                             | 0.15.4  | 41.78s     | 86,869,452 bytes     |
| vanilla-extract                    | 1.17.4  | 19.50s     | 61,494,033 bytes     |
| kuma-ui                            | 1.5.9   | 20.93s     | 69,924,179 bytes     |
| panda-css                          | 1.3.1   | 20.64s     | 64,573,260 bytes     |
| chakra-ui                          | 3.27.0  | 28.81s     | 222,435,802 bytes    |
| mui                                | 7.3.2   | 20.86s     | 97,964,458 bytes     |
| **devup-ui(per-file css)**         | 1.0.18  | **16.90s** | 59,540,459 bytes     |
| **devup-ui(single css)**           | 1.0.18  | **17.05s** | **59,520,196 bytes** |
| tailwindcss(turbopack)             | 4.1.13  | 6.72s      | 5,355,082 bytes      |
| **devup-ui(single css+turbopack)** | 1.0.18  | 10.34s     | **4,772,050 bytes**  |

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
