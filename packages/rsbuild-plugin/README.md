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

<!-- benchmark:start -->

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/37758611477) (commit `467a379a`, Devup UI 1.0.44) on `ubuntu-24.04` with Next.js 16.3.3. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`.

Build Size is every byte of the build output directory (`.next`, or `dist` for vinext), including caches and server output. The checked table retains its original historical CSS measurements, which recursively counted `.css` files throughout that output. New runs count only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.

Webpack:

| Library                     | Version | Median Build Time | Range           | Build Size        | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | ----------------- | ------------ |
| tailwindcss                 | 4.3.3   | 13.02s            | 12.91s - 13.20s | 67,339,250 bytes  | 5,852 bytes  |
| styleX                      | 0.19.0  | 29.01s            | 28.72s - 29.50s | 96,290,533 bytes  | 425 bytes    |
| vanilla-extract             | 1.21.2  | 12.34s            | 12.22s - 12.35s | 68,565,511 bytes  | 294 bytes    |
| kuma-ui                     | 1.6.4   | 13.55s            | 13.49s - 13.57s | 75,627,429 bytes  | 340 bytes    |
| panda-css                   | 1.12.0  | 13.44s            | 13.29s - 13.58s | 71,849,243 bytes  | 15,615 bytes |
| chakra-ui                   | 3.37.0  | 20.29s            | 19.95s - 21.06s | 207,425,028 bytes | 0 bytes      |
| mui                         | 9.4.0   | 14.12s            | 13.78s - 14.26s | 101,476,098 bytes | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **10.95s**        | 10.87s - 11.09s | 67,427,695 bytes  | 562 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **11.04s**        | 10.88s - 11.40s | 67,436,336 bytes  | 790 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Build Size       | CSS Size    |
| -------------------------------------- | ------- | ----------------- | ------------- | ---------------- | ----------- |
| tailwindcss                            | 4.3.3   | 5.68s             | 5.51s - 5.73s | 38,440,013 bytes | 6,197 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **5.79s**         | 5.77s - 5.87s | 36,538,765 bytes | 327 bytes   |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **5.71s**         | 5.67s - 5.99s | 36,580,207 bytes | 268 bytes   |

The Turbopack ranges do not overlap, so the direct-API median is 0.12s (2.1%) behind Tailwind on this fixture. The 3 cold samples were Tailwind `5.68s, 5.73s, 5.51s`, direct Devup UI `5.77s, 5.79s, 5.87s`, and static `.css.ts` `5.71s, 5.99s, 5.67s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
<!-- benchmark:end -->

## How it works

Devup UI is a CSS in JS preprocessor that does not require runtime.
Devup UI eliminates the performance degradation of the browser through the CSS in JS preprocessor.
We develop a preprocessor that considers all grammatical cases.

Generated class and CSS variable names may change between versions, build state, and prefix settings; do not manually reuse them.

```tsx
const before = <Box bg="red" />

const after = <div className="a" />
```

Variables are fully supported.

```tsx
const before = <Box bg={colorVariable} />

const after = (
  <div
    className="a"
    style={{
      '--b': colorVariable,
    }}
  />
)
```

Various expressions and responsiveness are also fully supported.

```tsx
const before = <Box bg={['red', 'blue', a > b ? 'yellow' : variable]} />

const after = (
  <div
    className={`a b ${a > b ? 'c' : 'd'}`}
    style={{
      '--e': variable,
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

## Compiler engine

Rsbuild uses the native compiler by default. Explicit selections are supported:

```ts
DevupUI() // native
DevupUI({ engine: 'native' })
DevupUI({ engine: 'wasm' })
```

Both engines are installed through plugin-utils. Keep optional dependencies
enabled for the native platform package. A native load failure names that
package and preserves the original error as its cause. There is no automatic
fallback; select `engine: 'wasm'` on hosts without a working native image or
required native libraries. Malformed shorthands fail before backend selection.

The selected compiler serves configuration, reservation, transforms and delayed
watch/build callbacks together. Neither engine supports concurrent independent
builds in the same process.

## License banner

Every emitted stylesheet starts with a
`/*! devup-ui v… | Apache License 2.0 | https://devup-ui.com */` comment, which
minifiers keep. It is on by default; `licenseBanner: false` drops it from every
stylesheet. A stylesheet without any rule is emitted empty, banner included.

```ts
DevupUI({ licenseBanner: false })
```

## Project root

Relative `devupFile`, `distDir` and `cssDir` paths, and the `tsconfig.json` the
plugin reads, resolve against Rsbuild's root (`api.context.rootPath`), not
against the directory the build is started from. A build that a monorepo tool
(turbo, nx, ...) runs from a parent directory reads `<root>/devup.json` and
writes `<root>/df`, as a build run from the project itself does. Builds run from
the project root are unchanged.

When `devupFile` is not set and the project root has no `devup.json` but the
working directory does, that file is still read and a one-time warning is
printed. Move `devup.json` into the project root, or set `devupFile` to the file
you want, to resolve it.

## Custom Shorthands

```ts
// rsbuild.config.ts
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

Custom shorthands are build-plugin options, not theme tokens. Every target
receives the same value, with camelCase and kebab-case target names supported.
The generated `df/theme.d.ts` provides type completion for component props,
responsive values, and selectors. Restart Rsbuild after changing the option.
If `tsconfig.json` only includes `src`, add `df/*.d.ts` (or your custom
`distDir`) to `include`.

```tsx
<Box insetX={[0, null, 'auto']} _hover={{ insetX: 4 }} />
```
