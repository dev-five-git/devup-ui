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

English | [한국어](README_ko.md)

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

- Preprocessor
- Zero Config
- Zero FOUC
- Zero Runtime
- RSC Support
- Must not use JavaScript, client-side logic, or hybrid solutions
- Support Library mode
- Zero Cost Dynamic Theme Support based on CSS Variables
- Theme with Typing
- Smallest size, fastest speed

## Inspirations

- Styled System
- Chakra UI
- Theme UI
- Vanilla Extract
- Rainbow Sprinkles
- Kuma UI

## Comparison Benchmarks

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/33239265133) on `ubuntu-latest`. All Next.js builds use the native TypeScript 7 CLI for type checking.

Webpack values are one cold build:

| Library                     | Version | Build Time | Build Size        |
| --------------------------- | ------- | ---------- | ----------------- |
| tailwindcss                 | 4.3.3   | 15.55s     | 66,479,450 bytes  |
| styleX                      | 0.19.0  | 34.27s     | 95,417,163 bytes  |
| vanilla-extract             | 1.21.2  | 14.91s     | 67,711,539 bytes  |
| kuma-ui                     | 1.6.4   | 16.43s     | 74,774,187 bytes  |
| panda-css                   | 1.12.0  | 16.82s     | 70,983,831 bytes  |
| chakra-ui                   | 3.37.0  | 24.79s     | 206,598,161 bytes |
| mui                         | 9.4.0   | 17.12s     | 100,621,370 bytes |
| **devup-ui (per-file CSS)** | 1.0.40  | **13.43s** | 66,577,087 bytes  |
| **devup-ui (single CSS)**   | 1.0.40  | **13.37s** | 66,564,796 bytes  |

Turbopack values are medians of six cold builds in alternating order:

| Library                                | Version | Median Build Time | Build Size           |
| -------------------------------------- | ------- | ----------------- | -------------------- |
| tailwindcss                            | 4.3.3   | 6.55s             | 38,386,955 bytes     |
| **devup-ui (direct APIs, single CSS)** | 1.0.40  | **6.54s**         | **36,519,234 bytes** |
| **devup-ui (static `.css.ts`)**        | 1.0.40  | **6.47s**         | 36,550,197 bytes     |

The Turbopack ranges overlap, so the direct-API result is effectively parity with Tailwind on this fixture. The static `.css.ts` row uses the `lite` WASM fast path; dynamic `.css.ts` modules use the full Boa evaluator and are not represented by that median.

## How it works

Devup UI is a CSS in JS preprocessor that does not require runtime.
Devup UI eliminates the performance degradation of the browser through the CSS in JS preprocessor.
We develop a preprocessor that considers all grammatical cases.

```tsx
const before = <Box bg="red" />

const after = <div className="d0" />
```

Variables are fully supported.

```tsx
const before = <Box bg={colorVariable} />

const after = (
  <div
    className="d0"
    style={{
      '--d0': colorVariable,
    }}
  />
)
```

Various expressions and responsiveness are also fully supported.

```tsx
const before = <Box bg={['red', 'blue', a > b ? 'yellow' : variable]} />

const after = (
  <div
    className={`d0 d1 ${a > b ? 'd2' : 'd3'}`}
    style={{
      '--d2': variable,
    }}
  />
)
```

Support Theme with Typing

`devup.json`

```json
{
  "theme": {
    "colors": {
      "default": {
        "text": "#000"
      },
      "dark": {
        "text": "white"
      }
    }
  }
}
```

```tsx
// Type Safe
<Text color="$text" />
```

## Custom Shorthand Types

Build plugins accept a `shorthands` option separately from `devup.json`. After
the plugin runs, it augments `DevupCustomShorthands` in `df/theme.d.ts` so the
configured names are available on `DevupProps` and nested selector props.

```ts
DevupUI({
  shorthands: {
    insetX: ['left', 'right'],
  },
})
```

```tsx
<Box
  insetX={[0, null, 'auto']}
  _hover={{ insetX: 4 }}
  selectors={{ '& > *': { insetX: 2 } }}
/>
```

Restart the build plugin after changing shorthand configuration so the
generated declaration is refreshed.
If `tsconfig.json` only includes `src`, add `df/*.d.ts` (or your plugin's custom
`distDir`) to `include`.

Support Responsive And Pseudo Selector

You can use responsive and pseudo selector.

```tsx
// Responsive with Selector
const box = <Box _hover={{ bg: ['red', 'blue'] }} />

// Same
const box = <Box _hover={[{ bg: 'red' }, { bg: 'blue' }]} />
```
