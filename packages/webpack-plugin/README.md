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

The Turbopack ranges overlap, so the direct-API result is effectively parity with Tailwind on this fixture. The static `.css.ts` row was measured with the former `lite` fast path for static modules; every `.css.ts` module now runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one.

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

Support Responsive And Pseudo Selector

You can use responsive and pseudo selector.

```tsx
// Responsive with Selector
const box = <Box _hover={{ bg: ['red', 'blue'] }} />

// Same
const box = <Box _hover={[{ bg: 'red' }, { bg: 'blue' }]} />
```

## Consuming Component Libraries

A precompiled Devup UI library ships transformed JavaScript and emitted CSS.
Import its published stylesheet; its compile-time components have already been
removed, so the consumer does not need to transform that library again.

A library intentionally published without extraction (for example with Vite's
`DevupUI({ extractCss: false })`, as in `apps/vite-lib`) requires a Devup UI build
plugin in **every** consuming application and its exact package name in `include`:

```ts
new DevupUIWebpackPlugin({ include: ['@acme/components'] })
```

The consumer compiles the library and emits its CSS at build time. Uncompiled
output is not standalone browser JavaScript: without this consumer setup, the
component placeholders throw `Cannot run on the runtime`. No styling runtime is
added by either supported publishing mode.

## MDX Selection And Compiled-Source Guards

Native `resolve.alias` maps or ordered descriptors are passed unchanged to the
shared graph, build-time resolver and loader; resolver cache identity includes
their order and values. Duplicate descriptors and literal-dollar names retain
native semantics. Empty candidate arrays perform no rewrite and fall through;
an actual rewriting all-miss is terminal. The installed enhanced-resolve factory
rejects mixed string/false arrays before its alias handler, so Webpack reports
its own construction error for those arrays. Wildcard names the shared resolver
cannot represent are located configuration errors, never silently dropped.

`mdxExtensions` defaults to `['.mdx']`. Configure a project MDX loader for every
literal extension you opt into, for example `['.mdx', '.mdown']`; `.md` is not
selected by default. Devup's post-loader extracts compiler-produced JavaScript
with JSX under the actual filename. Graph discovery and numbering use the same
selection as extraction.

With extraction enabled, the guard runs after loaders in build, development
compilations and watch rebuilds. Public parser facts, used dependency export IDs
and actual module targets identify definitely used compile-time exports in
modules outside the supported JS/TS and selected MDX extensions. Named bindings,
static namespace/require-result members and destructuring with known export IDs
produce a located module/import error and an `mdxExtensions` remedy. Runtime
helpers and unused imports are allowed; an intended runtime compatibility alias
can be disabled through `importAliases[package] = false`.

Whole namespace/require-result objects and dynamic imports are opaque and are
allowed. Compile-time exports reached only through these opaque forms may throw
`Cannot run on the runtime` when rendered; let Devup compile the extension
instead. The guard does not expand extraction to ordinary JS/TS excluded by
`include` or change intentional uncompiled-library publishing.

## Custom Shorthands

```ts
// webpack.config.ts
new DevupUIWebpackPlugin({
  shorthands: {
    insetX: ['left', 'right'],
    scrollMarginX: ['scrollMarginLeft', 'scrollMarginRight'],
  },
})
```

Custom shorthands extend build-time extraction and are intentionally separate
from `devup.json`. Every target receives the same value; camelCase and CSS
kebab-case target names are supported. After Webpack runs, the generated
`df/theme.d.ts` provides type completion on component props, responsive values,
and selectors. Restart Webpack after changing the option.
If `tsconfig.json` only includes `src`, add `df/*.d.ts` (or your custom
`distDir`) to `include`.

```tsx
<Box insetX={[0, null, 'auto']} _hover={{ insetX: 4 }} />
```
