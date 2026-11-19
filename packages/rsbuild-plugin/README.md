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

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/44481492291) (commit `4ee793d6`, Devup UI 1.0.44, 2026-11-19) on `ubuntu-26.04` with Next.js 16.4.6. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`, which CI refreshes after every merge to main and records in `benchmark/history.json`, one entry per commit, so a slower build can be traced to the merge that caused it.

Client Size is the JavaScript and CSS shipped to the browser, uncompressed: `.next/static/**/*.{js,css}` or vinext `dist/client/**/*.{js,css}`. Build Size (incl. caches) is every byte of the build output directory (`.next`, or `dist` for vinext); most of it is the bundler cache and server output, and it moves by a few kilobytes between identical builds, so it is kept for continuity rather than as what users download. CSS Size counts only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Every size is measured on every sample and shown as the median; `benchmark-results.json` keeps the samples with their range, and rows measured before Client Size was recorded show n/a. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.

Webpack:

| Library                     | Version | Median Build Time | Range           | Client Size     | Build Size (incl. caches) | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | --------------- | ------------------------- | ------------ |
| tailwindcss                 | 4.3.5   | 16.28s            | 16.17s - 16.72s | 924,205 bytes   | 69,757,094 bytes          | 717 bytes    |
| styleX                      | 0.19.2  | 33.64s            | 32.10s - 34.09s | 2,021,495 bytes | 97,728,565 bytes          | 661 bytes    |
| vanilla-extract             | 1.21.3  | 14.91s            | 14.32s - 15.00s | 923,744 bytes   | 70,451,393 bytes          | 398 bytes    |
| kuma-ui                     | 1.6.4   | 17.13s            | 16.61s - 17.16s | 956,179 bytes   | 77,758,536 bytes          | 354 bytes    |
| panda-css                   | 2.5.0   | 16.57s            | 16.46s - 16.70s | 980,304 bytes   | 75,833,921 bytes          | 18,567 bytes |
| chakra-ui                   | 3.38.0  | 24.98s            | 24.62s - 25.49s | 1,175,920 bytes | 211,190,705 bytes         | 0 bytes      |
| mui                         | 9.5.0   | 17.80s            | 17.43s - 18.10s | 993,420 bytes   | 103,207,307 bytes         | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **13.81s**        | 13.57s - 13.88s | 923,770 bytes   | 69,800,002 bytes          | 355 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **14.12s**        | 13.86s - 14.20s | 923,709 bytes   | 69,784,858 bytes          | 325 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Client Size   | Build Size (incl. caches) | CSS Size  |
| -------------------------------------- | ------- | ----------------- | ------------- | ------------- | ------------------------- | --------- |
| tailwindcss                            | 4.3.5   | 5.60s             | 5.48s - 5.74s | 571,690 bytes | 30,534,898 bytes          | 653 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **5.69s**         | 5.63s - 5.71s | 571,186 bytes | 28,909,760 bytes          | 325 bytes |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **5.68s**         | 5.65s - 5.82s | 571,283 bytes | 28,952,796 bytes          | 376 bytes |

The Turbopack ranges overlap, so the direct-API median is 0.09s (1.6%) behind Tailwind on this fixture and the two are effectively at parity. The 3 cold samples were Tailwind `5.48s, 5.74s, 5.60s`, direct Devup UI `5.69s, 5.63s, 5.71s`, and static `.css.ts` `5.68s, 5.65s, 5.82s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
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

## Library mode

A UI library written with Devup UI is published uncompiled: its output keeps
the Devup UI components, and the application that uses it compiles them. Build
the library with `extractCss: false`, so no class names or stylesheet
are generated for it:

```ts
// rsbuild.config.ts of the library
export default defineConfig({
  plugins: [DevupUI({ extractCss: false })],
})
```

Every application that uses the library must list the package in `include`.
The plugin skips packages in `node_modules` otherwise, so the library's
components stay uncompiled and throw `Cannot run on the runtime` when they
render. Each name matches that whole package (`pkg` is not `pkg-extra` or
`@scope/pkg`).

```ts
// rsbuild.config.ts of the application
export default defineConfig({
  plugins: [DevupUI({ include: ['your-ui-library'] })],
})
```

See [`apps/vite-lib`](https://github.com/dev-five-git/devup-ui/tree/main/apps/vite-lib)
for a library built this way, and the
[installation guide](https://devup-ui.com/docs/installation) for the setup of
each bundler.

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
