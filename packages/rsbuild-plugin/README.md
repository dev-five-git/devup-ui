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

## Publishing And Consuming Libraries

With the default `extractCss: true`, a library build extracts its styles. Publish
the transformed JavaScript together with its emitted CSS and expose a stylesheet
that consumers import. The published code no longer calls compile-time Devup UI
placeholders.

Use `extractCss: false` only for an intentionally uncompiled library. This is the
contract demonstrated by `apps/vite-lib`: **every** consumer needs a Devup UI build
plugin configured with the library's exact package name:

```ts
export default defineConfig({
  plugins: [DevupUI({ include: ['@acme/components'] })],
})
```

The consuming build transforms the library and emits its CSS. Without that
configuration, the uncompiled placeholders throw `Cannot run on the runtime`;
the library must not be treated as standalone browser JavaScript. Both publishing
modes remain free of a styling runtime.

## Project Roots And Extraction

Relative `devupFile`, `distDir`, and `cssDir` options resolve against Rsbuild's
`root`, not the shell's working directory. The graph includes existing `src`
and `app` directories, normalized Rspack entries, and `sourceDirs` (a directory
or array of directories). Included packages follow the same effective Rspack
ES-module export conditions as extraction.

Extraction supports `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, and
`.cjs`. Configure an MDX compiler for `.mdx`; Devup UI runs as a post-loader on
its compiled JavaScript, never on raw Markdown. Rsbuild's transform API does not
expose the incoming source map, so extraction errors label compiled MDX locations
with `(in compiled MDX)`.

Malformed or cyclic configuration and graph/prewarm failures stop the build with
the root and underlying cause. Empty themes overwrite generated declarations.
Optional file-number seeding failures warn once that IDs may depend on module
arrival order. `atomHoist` composes its shared CSS cache group with existing
groups and creates `splitChunks` when absent; `splitChunks: false` is an error
when atom hoisting is requested.

## MDX Selection And Compiled-Source Guards

The graph and build-time resolver receive the same native Rspack `resolve.alias`
map-or-false option. Rspack accepts ordered mixed string/false candidates:
the first resolving file wins, and a reached false ignores the module. Empty
arrays fall through to the raw request; a real rewriting all-miss remains
terminal. Rspack's public alias type does not accept Webpack-style descriptor
arrays. Unsupported wildcard names fail with the importer/key instead of being
silently dropped.

`mdxExtensions` defaults to `['.mdx']`; `.md` and custom literal extensions such
as `.mdown` are opt-in. Configure the project MDX loader for the same list.
Devup runs on its compiled JavaScript/JSX in a post-loader, retaining the actual
filename. Extraction, graph discovery and numbering share the selected list.

With extraction enabled, build, dev-server compilations and watch rebuilds use
Rspack's public used-dependency export IDs and actual module targets. A
definitely named compile-time export used by a module outside the supported
JS/TS or selected MDX extensions produces a located module/import error and an
`mdxExtensions` remedy. Static namespace/require member uses remain guarded when
the dependency names the export. Runtime helpers and unused imports are allowed.
For an intended runtime compatibility alias, set `importAliases[package] = false`.

Whole namespace/require-result objects and dynamic imports are opaque and are
allowed. Rspack additionally exposes some namespace/require destructuring uses
with `ids = []`, and the installed Rspack may omit export IDs for CommonJS
require members, bound results and destructuring altogether. Some renamed
forwarding cannot be traced through its public export data. Those forms are
also opaque and allowed, even though another
bundler with definite export IDs can reject the same source. No provided-export
query is made at `finishModules`. Hidden compile-time exports can still throw
`Cannot run on the runtime` when rendered; add the extension to `mdxExtensions`
and let Devup compile it. The guard does not expand ordinary JS/TS `include`
selection or change `extractCss: false` publishing.

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
