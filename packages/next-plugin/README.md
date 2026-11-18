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

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/44324594883) (commit `a2bc2863`, Devup UI 1.0.44, 2026-11-18) on `ubuntu-26.04` with Next.js 16.4.6. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`, which CI refreshes after every merge to main and records in `benchmark/history.json`, one entry per commit, so a slower build can be traced to the merge that caused it.

Client Size is the JavaScript and CSS shipped to the browser, uncompressed: `.next/static/**/*.{js,css}` or vinext `dist/client/**/*.{js,css}`. Build Size (incl. caches) is every byte of the build output directory (`.next`, or `dist` for vinext); most of it is the bundler cache and server output, and it moves by a few kilobytes between identical builds, so it is kept for continuity rather than as what users download. CSS Size counts only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Every size is measured on every sample and shown as the median; `benchmark-results.json` keeps the samples with their range, and rows measured before Client Size was recorded show n/a. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.

Webpack:

| Library                     | Version | Median Build Time | Range           | Client Size     | Build Size (incl. caches) | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | --------------- | ------------------------- | ------------ |
| tailwindcss                 | 4.3.5   | 9.72s             | 9.63s - 9.82s   | 924,183 bytes   | 69,759,179 bytes          | 717 bytes    |
| styleX                      | 0.19.2  | 20.66s            | 20.60s - 20.77s | 2,021,525 bytes | 97,728,826 bytes          | 661 bytes    |
| vanilla-extract             | 1.21.3  | 8.75s             | 8.72s - 8.79s   | 923,721 bytes   | 70,448,255 bytes          | 398 bytes    |
| kuma-ui                     | 1.6.4   | 10.11s            | 10.04s - 10.13s | 956,158 bytes   | 77,758,260 bytes          | 354 bytes    |
| panda-css                   | 2.5.0   | 10.08s            | 10.07s - 10.10s | 980,282 bytes   | 75,834,079 bytes          | 18,567 bytes |
| chakra-ui                   | 3.38.0  | 15.21s            | 15.20s - 15.29s | 1,175,910 bytes | 211,191,178 bytes         | 0 bytes      |
| mui                         | 9.5.0   | 10.82s            | 10.82s - 10.87s | 993,400 bytes   | 103,207,812 bytes         | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **8.50s**         | 8.47s - 8.52s   | 923,753 bytes   | 69,829,934 bytes          | 355 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **8.49s**         | 8.47s - 8.53s   | 923,689 bytes   | 69,817,584 bytes          | 325 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Client Size   | Build Size (incl. caches) | CSS Size  |
| -------------------------------------- | ------- | ----------------- | ------------- | ------------- | ------------------------- | --------- |
| tailwindcss                            | 4.3.5   | 3.77s             | 3.77s - 3.81s | 571,738 bytes | 30,525,154 bytes          | 653 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **3.77s**         | 3.76s - 3.78s | 571,234 bytes | 28,921,641 bytes          | 325 bytes |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **3.74s**         | 3.68s - 3.75s | 571,331 bytes | 28,964,492 bytes          | 376 bytes |

The Turbopack ranges overlap, so the direct-API median is 0.00s (0.1%) ahead of Tailwind on this fixture and the two are effectively at parity. The 3 cold samples were Tailwind `3.77s, 3.77s, 3.81s`, direct Devup UI `3.76s, 3.78s, 3.77s`, and static `.css.ts` `3.74s, 3.75s, 3.68s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
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

## Project root

With webpack (`next build --webpack`), relative `devupFile`, `distDir` and
`cssDir` paths, the `tsconfig.json` the plugin reads and the file ids class names
are derived from resolve against the Next project directory, not against the
directory the build is started from. A build that a monorepo tool (turbo, nx,
...) runs from a parent directory (`next build apps/web`) reads
`apps/web/devup.json` and writes `apps/web/df`. Builds run from the project
directory are unchanged; builds run from a parent directory may get different
class names once. Turbopack does not tell its config the project directory, so
there the paths stay relative to the working directory.

When `devupFile` is not set and the project directory has no `devup.json` but
the working directory does, that file is still read and a one-time warning is
printed. Move `devup.json` into the project directory, or set `devupFile` to the
file you want, to resolve it.

## Custom Shorthands

Custom shorthands belong to the plugin options rather than `devup.json` because
they extend build-time extraction, not design tokens.

```ts
// next.config.ts
const nextConfig = {}

export default DevupUI(nextConfig, {
  shorthands: {
    insetX: ['left', 'right'],
    scrollMarginX: ['scrollMarginLeft', 'scrollMarginRight'],
  },
})
```

Every target receives the same value, and target names accept camelCase or
kebab-case. The generated `df/theme.d.ts` adds type completion for direct,
responsive, pseudo-selector, and custom-selector usage. Restart Next.js after
changing the option.
If your `tsconfig.json` does not already match `df/*.d.ts`, add it (or your
custom `distDir`) to `include`.

```tsx
<Box insetX={[0, null, 'auto']} _hover={{ insetX: 4 }} />
```

## Extraction engine

The build-time compiler defaults to `native` for Webpack and Turbopack. Select
WASM explicitly with `DevupUI(nextConfig, { engine: 'wasm' })`. The chosen engine
is retained by configuration, prewarm, extraction, CSS and delayed callbacks.
Turbopack loader options carry only the engine string; the coordinator retains
the selected physical compiler. Turbopack apps in one process keep one
coordinator and one engine session per resolved `distDir`, so their numbering,
sheet, theme and settings stay apart; give each app its own `distDir`. A config
re-evaluation of the same app replaces only that app's coordinator. Independent
concurrent Webpack builds in one process are unsupported.

Native load failures identify the required platform package and retain the
original cause. There is no automatic fallback to WASM. Install optional native
dependencies normally, or choose WASM explicitly for an unsupported platform.
Both engines extract CSS during the build and add no client styling runtime.

## License banner

Every emitted stylesheet starts with a
`/*! devup-ui v… | Apache License 2.0 | https://devup-ui.com */` comment, which
minifiers keep. It is on by default; `licenseBanner: false` drops it from every
stylesheet, under Webpack and Turbopack alike. A stylesheet without any rule is
emitted empty, banner included.

```ts
export default DevupUI(nextConfig, { licenseBanner: false })
```

## MDX

With `@next/mdx`, Devup UI components and APIs used directly in an `.mdx`
document are extracted from the compiled document, under webpack and Turbopack
alike. The wrapping order does not matter:

```ts
export default createMDX()(DevupUI(nextConfig))
// or
export default DevupUI(createMDX()(nextConfig))
```

`@next/mdx` hands Turbopack its output as a `.tsx` module, which runs through
the Devup UI source rule. A production Turbopack build cannot extract a
document before it compiles, so its stylesheet waits for the MDX route files
(when `pageExtensions` lists `mdx`) and for the documents those and the
compiled sources import by a relative path. A document imported only through a
path alias is not waited for.

## Turbopack build profiling

Set `DEVUP_UI_PROFILE=1` for an opt-in, structured timing log during a
Turbopack build.

```bash
DEVUP_UI_PROFILE=1 bun run build
```

```powershell
$env:DEVUP_UI_PROFILE = '1'; bun run build
```

Each `[devup-ui:profile]` JSON entry reports one phase. `next.graph` measures
the static import-graph pre-pass, `next.prewarm` measures production extraction
before Turbopack starts loaders, and `coordinator.extract` separates request,
compiler extraction, CSS/state serialization, and write time for each loaded
module. The setting is disabled by default and does not collect timings or
write logs when it is absent.
