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

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/44324447924) (commit `14819ae1`, Devup UI 1.0.44, 2026-11-18) on `ubuntu-26.04` with Next.js 16.4.6. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`, which CI refreshes after every merge to main and records in `benchmark/history.json`, one entry per commit, so a slower build can be traced to the merge that caused it.

Client Size is the JavaScript and CSS shipped to the browser, uncompressed: `.next/static/**/*.{js,css}` or vinext `dist/client/**/*.{js,css}`. Build Size (incl. caches) is every byte of the build output directory (`.next`, or `dist` for vinext); most of it is the bundler cache and server output, and it moves by a few kilobytes between identical builds, so it is kept for continuity rather than as what users download. CSS Size counts only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Every size is measured on every sample and shown as the median; `benchmark-results.json` keeps the samples with their range, and rows measured before Client Size was recorded show n/a. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.

Webpack:

| Library                     | Version | Median Build Time | Range           | Client Size     | Build Size (incl. caches) | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | --------------- | ------------------------- | ------------ |
| tailwindcss                 | 4.3.5   | 11.94s            | 11.86s - 12.02s | 924,183 bytes   | 69,758,622 bytes          | 717 bytes    |
| styleX                      | 0.19.2  | 24.36s            | 24.14s - 24.52s | 2,021,525 bytes | 97,727,611 bytes          | 661 bytes    |
| vanilla-extract             | 1.21.3  | 10.82s            | 10.79s - 10.85s | 923,721 bytes   | 70,448,102 bytes          | 398 bytes    |
| kuma-ui                     | 1.6.4   | 12.43s            | 12.42s - 12.46s | 956,158 bytes   | 77,760,538 bytes          | 354 bytes    |
| panda-css                   | 2.5.0   | 12.28s            | 12.23s - 12.32s | 980,282 bytes   | 75,835,577 bytes          | 18,567 bytes |
| chakra-ui                   | 3.38.0  | 18.52s            | 18.52s - 18.64s | 1,175,910 bytes | 211,192,005 bytes         | 0 bytes      |
| mui                         | 9.5.0   | 13.14s            | 13.12s - 13.36s | 993,400 bytes   | 103,205,651 bytes         | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **10.50s**        | 10.43s - 10.86s | 923,753 bytes   | 69,830,734 bytes          | 355 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **10.55s**        | 10.49s - 10.82s | 923,689 bytes   | 69,814,786 bytes          | 325 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Client Size   | Build Size (incl. caches) | CSS Size  |
| -------------------------------------- | ------- | ----------------- | ------------- | ------------- | ------------------------- | --------- |
| tailwindcss                            | 4.3.5   | 4.42s             | 4.33s - 4.44s | 571,738 bytes | 30,530,536 bytes          | 653 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **4.26s**         | 4.24s - 4.35s | 571,234 bytes | 28,917,131 bytes          | 325 bytes |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **4.31s**         | 4.29s - 4.38s | 571,331 bytes | 28,972,453 bytes          | 376 bytes |

The Turbopack ranges overlap, so the direct-API median is 0.16s (3.7%) ahead of Tailwind on this fixture and the two are effectively at parity. The 3 cold samples were Tailwind `4.44s, 4.42s, 4.33s`, direct Devup UI `4.35s, 4.24s, 4.26s`, and static `.css.ts` `4.38s, 4.29s, 4.31s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
<!-- benchmark:end -->

## How it works

Devup UI is a CSS in JS preprocessor that does not require runtime.
Devup UI eliminates the performance degradation of the browser through the CSS in JS preprocessor.
What the build cannot know is kept as a CSS variable or reported as a [located build error](https://devup-ui.com/docs/build-errors); see [supported syntax & limitations](https://devup-ui.com/docs/limitations).

Generated class and CSS variable names may change between versions, build state, and prefix settings; do not manually reuse them.

```tsx
const before = <Box bg="red" />

const after = <div className="a" />
```

Variables become CSS variables.

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

Conditions and responsive arrays compile too.

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
