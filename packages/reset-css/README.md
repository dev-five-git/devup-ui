<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor
</h3>

---

<div>
<img src='https://img.shields.io/npm/v/@devup-ui/reset-css'>
<img src='https://img.shields.io/bundlephobia/minzip/@devup-ui/reset-css'>
<img alt="Github Checks" src="https://badgen.net/github/checks/dev-five-git/devup-ui"/>
<img alt="Apache-2.0 License" src="https://img.shields.io/github/license/dev-five-git/devup-ui"/>
<a href="https://www.npmjs.com/package/@devup-ui/reset-css">
<img alt="NPM Downloads" src="https://img.shields.io/npm/dm/@devup-ui/reset-css.svg?style=flat"/>
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
npm install @devup-ui/reset-css @devup-ui/react

# on next.js
npm install @devup-ui/next-plugin

# on vite
npm install @devup-ui/vite-plugin

# on rsbuild
npm install @devup-ui/rsbuild-plugin

# on webpack
npm install @devup-ui/webpack-plugin
```

## Usage

`@devup-ui/reset-css` is a CSS reset based on
[sanitize.css](https://github.com/csstools/sanitize.css), written with
`globalCss()` and compiled into your stylesheet by the Devup UI build plugin.
Import it once near the root of your app and call `resetCss()`:

```tsx
import { resetCss } from '@devup-ui/reset-css'

resetCss()
```

Importing the module adds the rules; `resetCss()` does nothing at runtime and
only keeps the import from being removed as unused. Most selectors are wrapped
in `:where()`, so your own styles override them.

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

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/44438491048) (commit `55cd8e34`, Devup UI 1.0.44, 2026-11-19) on `ubuntu-26.04` with Next.js 16.4.6. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`, which CI refreshes after every merge to main and records in `benchmark/history.json`, one entry per commit, so a slower build can be traced to the merge that caused it.

Client Size is the JavaScript and CSS shipped to the browser, uncompressed: `.next/static/**/*.{js,css}` or vinext `dist/client/**/*.{js,css}`. Build Size (incl. caches) is every byte of the build output directory (`.next`, or `dist` for vinext); most of it is the bundler cache and server output, and it moves by a few kilobytes between identical builds, so it is kept for continuity rather than as what users download. CSS Size counts only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Every size is measured on every sample and shown as the median; `benchmark-results.json` keeps the samples with their range, and rows measured before Client Size was recorded show n/a. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.

Webpack:

| Library                     | Version | Median Build Time | Range           | Client Size     | Build Size (incl. caches) | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | --------------- | ------------------------- | ------------ |
| tailwindcss                 | 4.3.5   | 9.73s             | 9.65s - 10.05s  | 924,205 bytes   | 69,756,665 bytes          | 717 bytes    |
| styleX                      | 0.19.2  | 20.77s            | 20.50s - 20.91s | 2,021,495 bytes | 97,727,620 bytes          | 661 bytes    |
| vanilla-extract             | 1.21.3  | 8.76s             | 8.65s - 8.79s   | 923,744 bytes   | 70,450,949 bytes          | 398 bytes    |
| kuma-ui                     | 1.6.4   | 10.18s            | 10.10s - 10.26s | 956,179 bytes   | 77,759,053 bytes          | 354 bytes    |
| panda-css                   | 2.5.0   | 10.16s            | 10.04s - 10.30s | 980,304 bytes   | 75,832,738 bytes          | 18,567 bytes |
| chakra-ui                   | 3.38.0  | 15.28s            | 15.15s - 15.49s | 1,175,920 bytes | 211,190,700 bytes         | 0 bytes      |
| mui                         | 9.5.0   | 10.92s            | 10.81s - 11.00s | 993,420 bytes   | 103,202,372 bytes         | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **8.61s**         | 8.43s - 8.63s   | 923,768 bytes   | 69,829,835 bytes          | 355 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **8.50s**         | 8.50s - 8.67s   | 923,709 bytes   | 69,817,551 bytes          | 325 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Client Size   | Build Size (incl. caches) | CSS Size  |
| -------------------------------------- | ------- | ----------------- | ------------- | ------------- | ------------------------- | --------- |
| tailwindcss                            | 4.3.5   | 3.77s             | 3.76s - 3.83s | 571,690 bytes | 30,542,711 bytes          | 653 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **3.74s**         | 3.67s - 3.81s | 571,186 bytes | 28,921,152 bytes          | 325 bytes |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **3.71s**         | 3.68s - 3.79s | 571,283 bytes | 28,969,533 bytes          | 376 bytes |

The Turbopack ranges overlap, so the direct-API median is 0.03s (0.9%) ahead of Tailwind on this fixture and the two are effectively at parity. The 3 cold samples were Tailwind `3.83s, 3.77s, 3.76s`, direct Devup UI `3.81s, 3.67s, 3.74s`, and static `.css.ts` `3.79s, 3.68s, 3.71s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
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
