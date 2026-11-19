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

[Latest CI benchmark](https://github.com/dev-five-git/devup-ui/actions/runs/44481820252) (commit `9a64f7c9`, Devup UI 1.0.44, 2026-11-19) on `ubuntu-26.04` with Next.js 16.4.6. Every row is the median of 3 cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, `benchmark-results.json`, which CI refreshes after every merge to main and records in `benchmark/history.json`, one entry per commit, so a slower build can be traced to the merge that caused it.

Client Size is the JavaScript and CSS shipped to the browser, uncompressed: `.next/static/**/*.{js,css}` or vinext `dist/client/**/*.{js,css}`. Build Size is the deployable build output, uncompressed: what a deployment uploads, never the caches and type-check files a build leaves next to it. For Next.js it is `.next` without `cache/` (the Webpack/Turbopack and SWC caches and TypeScript's `.tsbuildinfo`), Turbopack's build-time `build/`, the generated `types/`, `diagnostics/`, the `trace` files, the output-tracing `*.nft.json` records and source maps, which is about what Next.js's `output: 'standalone'` copies plus `.next/static`; for vinext it is `dist` without source maps. The rule is `isDeployed` in `benchmark-output.js`; history entries recorded before it (no `bytesDefinition` in `benchmark/history.json`) counted the whole output directory. CSS Size counts only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Every size is measured on every sample and shown as the median; `benchmark-results.json` keeps the samples with their range, and rows measured before Client Size was recorded show n/a. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.

Webpack:

| Library                     | Version | Median Build Time | Range           | Client Size     | Build Size      | CSS Size     |
| --------------------------- | ------- | ----------------- | --------------- | --------------- | --------------- | ------------ |
| tailwindcss                 | 4.3.5   | 12.24s            | 12.17s - 12.37s | 924,205 bytes   | 1,796,355 bytes | 717 bytes    |
| styleX                      | 0.19.2  | 25.13s            | 24.51s - 25.39s | 2,021,495 bytes | 2,828,421 bytes | 661 bytes    |
| vanilla-extract             | 1.21.3  | 11.15s            | 11.06s - 11.22s | 923,744 bytes   | 1,795,160 bytes | 398 bytes    |
| kuma-ui                     | 1.6.4   | 12.90s            | 12.74s - 12.99s | 956,179 bytes   | 1,858,609 bytes | 354 bytes    |
| panda-css                   | 2.5.0   | 12.86s            | 12.47s - 12.89s | 980,304 bytes   | 1,890,730 bytes | 18,567 bytes |
| chakra-ui                   | 3.38.0  | 19.08s            | 18.36s - 19.35s | 1,175,920 bytes | 2,441,133 bytes | 0 bytes      |
| mui                         | 9.5.0   | 13.70s            | 13.16s - 13.85s | 993,420 bytes   | 1,934,375 bytes | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44  | **10.77s**        | 10.51s - 10.87s | 923,709 bytes   | 1,793,372 bytes | 325 bytes    |
| **devup-ui (single CSS)**   | 1.0.44  | **10.85s**        | 10.65s - 10.93s | 923,709 bytes   | 1,793,540 bytes | 325 bytes    |

Turbopack:

| Library                                | Version | Median Build Time | Range         | Client Size   | Build Size      | CSS Size  |
| -------------------------------------- | ------- | ----------------- | ------------- | ------------- | --------------- | --------- |
| tailwindcss                            | 4.3.5   | 4.54s             | 4.54s - 4.70s | 571,690 bytes | 1,368,423 bytes | 653 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44  | **4.43s**         | 4.43s - 4.47s | 571,186 bytes | 1,366,865 bytes | 325 bytes |
| **devup-ui (static `.css.ts`)**        | 1.0.44  | **4.42s**         | 4.42s - 4.67s | 571,283 bytes | 1,367,113 bytes | 376 bytes |

The Turbopack ranges do not overlap, so the direct-API median is 0.11s (2.4%) ahead of Tailwind on this fixture. The 3 cold samples were Tailwind `4.70s, 4.54s, 4.54s`, direct Devup UI `4.47s, 4.43s, 4.43s`, and static `.css.ts` `4.67s, 4.42s, 4.42s`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every `.css.ts` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see `benchmark-gate.js` for the rule) and fails on a regression beyond the measured noise.
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
Every Turbopack loader asks the coordinator, which retains the selected physical
compiler, so the loader options carry only the coordinator port file, the theme
files (relative to the project root) and a short key of the setup state.

Next.js copies its resolved config, these two loader rules included, into
`.next/required-server-files.{js,json}` and the standalone `server.js`. The
rules add about 500 bytes there, the same for any app size, with no local path.
They cannot be removed safely today: Next.js copies the config before any hook
the plugin can register runs, and its only option that leaves bundler settings
out of that copy also changes how the server reads its deployment id
([vercel/next.js#102045](https://github.com/vercel/next.js/discussions/102045)). Turbopack apps in one process keep one
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
