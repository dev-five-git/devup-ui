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

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/44498646216) (commit `3594eb3b`, Devup UI 1.0.44, 2026-11-19) on `ubuntu-26.04` with Next.js 16.4.6. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`, which CI refreshes after every merge to main and records in `benchmark/history.json`, one entry per commit, so a slower build can be traced to the merge that caused it.

Client Size is the JavaScript and CSS shipped to the browser, uncompressed: `.next/static/**/*.{js,css}` or vinext `dist/client/**/*.{js,css}`. Build Size is the deployable build output, uncompressed: what a deployment uploads, never the caches and type-check files a build leaves next to it. For Next.js it is `.next` without `cache/` (the Webpack/Turbopack and SWC caches and TypeScript's `.tsbuildinfo`), Turbopack's build-time `build/`, the generated `types/`, `diagnostics/`, the `trace` files, the output-tracing `*.nft.json` records and source maps, which is about what Next.js's `output: 'standalone'` copies plus `.next/static`; for vinext it is `dist` without source maps. The rule is `isDeployed` in `benchmark-output.js`; history entries recorded before it (no `bytesDefinition` in `benchmark/history.json`) counted the whole output directory. CSS Size counts only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Every size is measured on every sample and shown as the median; `benchmark-results.json` keeps the samples with their range, and rows measured before Client Size was recorded show n/a. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.

Webpack:

| Library                     | Version | Median Build Time | Range           | Client Size     | Build Size      | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | --------------- | --------------- | ------------ |
| tailwindcss                 | 4.3.5   | 10.19s            | 10.09s - 10.52s | 924,205 bytes   | 1,796,355 bytes | 717 bytes    |
| styleX                      | 0.19.2  | 21.87s            | 21.80s - 22.19s | 2,021,495 bytes | 2,828,421 bytes | 661 bytes    |
| vanilla-extract             | 1.21.3  | 9.23s             | 9.18s - 9.45s   | 923,744 bytes   | 1,795,160 bytes | 398 bytes    |
| kuma-ui                     | 1.6.4   | 10.69s            | 10.35s - 10.69s | 956,179 bytes   | 1,858,609 bytes | 354 bytes    |
| panda-css                   | 2.5.0   | 10.65s            | 10.48s - 10.68s | 980,304 bytes   | 1,890,730 bytes | 18,567 bytes |
| chakra-ui                   | 3.38.0  | 16.43s            | 16.22s - 16.56s | 1,175,920 bytes | 2,441,133 bytes | 0 bytes      |
| mui                         | 9.5.0   | 11.53s            | 11.38s - 11.55s | 993,420 bytes   | 1,934,375 bytes | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **8.92s**         | 8.91s - 9.03s   | 923,709 bytes   | 1,793,372 bytes | 325 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **8.96s**         | 8.81s - 9.01s   | 923,709 bytes   | 1,793,540 bytes | 325 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Client Size   | Build Size      | CSS Size  |
| -------------------------------------- | ------- | ----------------- | ------------- | ------------- | --------------- | --------- |
| tailwindcss                            | 4.3.5   | 3.91s             | 3.87s - 3.92s | 571,690 bytes | 1,368,423 bytes | 653 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **3.87s**         | 3.84s - 3.88s | 571,186 bytes | 1,366,865 bytes | 325 bytes |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **3.85s**         | 3.79s - 3.89s | 571,283 bytes | 1,367,113 bytes | 376 bytes |

The Turbopack ranges overlap, so the direct-API median is 0.03s (0.8%) ahead of Tailwind on this fixture and the two are effectively at parity. The 3 cold samples were Tailwind `3.92s, 3.87s, 3.91s`, direct Devup UI `3.84s, 3.87s, 3.88s`, and static `.css.ts` `3.89s, 3.79s, 3.85s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
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
const before = <Box color={colorVariable} />

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
const before = <Box color={['red', 'blue', a > b ? 'yellow' : variable]} />

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
