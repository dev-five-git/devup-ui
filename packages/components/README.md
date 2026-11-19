<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor
</h3>

---

<div>
<img src='https://img.shields.io/npm/v/@devup-ui/components'>
<img src='https://img.shields.io/bundlephobia/minzip/@devup-ui/components'>
<img alt="Github Checks" src="https://badgen.net/github/checks/dev-five-git/devup-ui"/>
<img alt="Apache-2.0 License" src="https://img.shields.io/github/license/dev-five-git/devup-ui"/>
<a href="https://www.npmjs.com/package/@devup-ui/components">
<img alt="NPM Downloads" src="https://img.shields.io/npm/dm/@devup-ui/components.svg?style=flat"/>
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
npm install @devup-ui/components @devup-ui/react

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

`@devup-ui/components` ships ready-made components (`Button`, `Checkbox`,
`Input`, `ProgressBar`, `Radio`, `RadioGroup`, `Search`, `Select`, `Stepper`,
`Textarea`, `Toggle`).
Their styles are compiled by the Devup UI build plugin like your own code; the
plugins compile `@devup-ui/*` packages in `node_modules` by default.

```tsx
import { Button, Input, Toggle } from '@devup-ui/components'

export function Form() {
  return (
    <form>
      <Input aria-label="Email" name="email" placeholder="Email" />
      <Toggle aria-label="Subscribe" name="subscribe" />
      <Button type="submit" variant="primary">
        Submit
      </Button>
    </form>
  )
}
```

See the [component documentation](https://devup-ui.com/components/overview)
for each component's props.

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

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/44463112815) (commit `6b54e2af`, Devup UI 1.0.44, 2026-11-19) on `ubuntu-26.04` with Next.js 16.4.6. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`, which CI refreshes after every merge to main and records in `benchmark/history.json`, one entry per commit, so a slower build can be traced to the merge that caused it.

Client Size is the JavaScript and CSS shipped to the browser, uncompressed: `.next/static/**/*.{js,css}` or vinext `dist/client/**/*.{js,css}`. Build Size (incl. caches) is every byte of the build output directory (`.next`, or `dist` for vinext); most of it is the bundler cache and server output, and it moves by a few kilobytes between identical builds, so it is kept for continuity rather than as what users download. CSS Size counts only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Every size is measured on every sample and shown as the median; `benchmark-results.json` keeps the samples with their range, and rows measured before Client Size was recorded show n/a. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.

Webpack:

| Library                     | Version | Median Build Time | Range           | Client Size     | Build Size (incl. caches) | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | --------------- | ------------------------- | ------------ |
| tailwindcss                 | 4.3.5   | 10.13s            | 10.07s - 10.22s | 924,205 bytes   | 69,757,010 bytes          | 717 bytes    |
| styleX                      | 0.19.2  | 22.02s            | 21.54s - 22.13s | 2,021,495 bytes | 97,727,016 bytes          | 661 bytes    |
| vanilla-extract             | 1.21.3  | 9.11s             | 9.03s - 9.15s   | 923,744 bytes   | 70,450,540 bytes          | 398 bytes    |
| kuma-ui                     | 1.6.4   | 10.49s            | 10.35s - 10.61s | 956,179 bytes   | 77,757,835 bytes          | 354 bytes    |
| panda-css                   | 2.5.0   | 10.51s            | 10.45s - 10.78s | 980,304 bytes   | 75,833,559 bytes          | 18,567 bytes |
| chakra-ui                   | 3.38.0  | 16.47s            | 16.13s - 16.57s | 1,175,920 bytes | 211,185,522 bytes         | 0 bytes      |
| mui                         | 9.5.0   | 11.50s            | 11.33s - 11.56s | 993,420 bytes   | 103,205,927 bytes         | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **8.81s**         | 8.80s - 9.09s   | 923,770 bytes   | 69,799,555 bytes          | 355 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **8.90s**         | 8.79s - 8.94s   | 923,709 bytes   | 69,787,074 bytes          | 325 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Client Size   | Build Size (incl. caches) | CSS Size  |
| -------------------------------------- | ------- | ----------------- | ------------- | ------------- | ------------------------- | --------- |
| tailwindcss                            | 4.3.5   | 3.90s             | 3.87s - 3.96s | 571,690 bytes | 30,539,321 bytes          | 653 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **3.93s**         | 3.82s - 3.95s | 571,186 bytes | 28,920,576 bytes          | 325 bytes |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **3.87s**         | 3.82s - 3.88s | 571,283 bytes | 28,967,613 bytes          | 376 bytes |

The Turbopack ranges overlap, so the direct-API median is 0.03s (0.7%) behind Tailwind on this fixture and the two are effectively at parity. The 3 cold samples were Tailwind `3.96s, 3.87s, 3.90s`, direct Devup UI `3.95s, 3.82s, 3.93s`, and static `.css.ts` `3.87s, 3.82s, 3.88s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
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
