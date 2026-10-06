<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor for Bun
</h3>

---

<div>
<img src='https://img.shields.io/npm/v/@devup-ui/bun-plugin'>
<img alt="Apache-2.0 License" src="https://img.shields.io/github/license/dev-five-git/devup-ui"/>
<a href="https://www.npmjs.com/package/@devup-ui/bun-plugin">
<img alt="NPM Downloads" src="https://img.shields.io/npm/dm/@devup-ui/bun-plugin.svg?style=flat"/>
</a>
<a href="https://badgen.net/github/stars/dev-five-git/devup-ui">
<img alt="Github Stars" src="https://badgen.net/github/stars/dev-five-git/devup-ui" />
</a>
<a href="https://discord.gg/8zjcGc7cWh">
<img alt="Discord" src="https://img.shields.io/discord/1321362173619994644.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2" />
</a>
</div>

---

## Install

```sh
bun add @devup-ui/react @devup-ui/bun-plugin
```

## Usage

Add the zero-config entry to Bun's preload list:

```toml
# bunfig.toml
[test]
preload = ["@devup-ui/bun-plugin"]
```

## Bundling with `Bun.build`

```ts
import { DevupUI } from '@devup-ui/bun-plugin/register'

await Bun.build({
  entrypoints: ['./src/index.tsx'],
  outdir: './dist',
  plugins: [DevupUI()],
})
```

The build emits the stylesheet as a CSS output holding the styles of every
module in the bundle. Imports of the packages Devup UI takes the place of
(`@emotion/react`, `@emotion/styled`, `styled-components`,
`@vanilla-extract/css`) and of `@stylexjs/stylex` are compiled too.

Class names are short by default in both `Bun.build` and the runtime. Pass
`debug: true` for readable names (including snapshot tests).

The plugin transforms loaded `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`,
`.mjs`, and `.cjs` modules, including libraries that import Devup UI, its
compatibility aliases, or StyleX. Unlike the other bundler plugins, Bun does
not require an `include` list when bundling loaded uncompiled libraries.
Under the runtime, use `include: ['your-library']` for uncompiled dependencies;
`@devup-ui/components` is included automatically. Runtime candidates are
discovered at setup so ordinary token/CommonJS modules retain Bun's native
watch/loading behavior; restart Bun after adding a new styling module or
changing `include`. Precompiled libraries must still publish their extracted
CSS for consumers to import.

`mdxExtensions` defaults to `['.mdx']`; `.md` and custom literal extensions such
as `.mdown` are opt-in. Devup compiles every selected file asynchronously using
the project's own `@mdx-js/mdx`, with explicit MDX format, before extraction.
Install that compiler even for plain selected MDX: missing compilation is a
located installation error, not a silent handoff to another plugin. The same
list controls loading, graph discovery and numbering. Compiled JavaScript/JSX
keeps its real filename. Extractor errors use the compiler's source map when
available, otherwise their locations are labeled `in compiled MDX`.

`root` defaults to `Bun.build`'s root, or the runtime's working directory.
Relative `devupFile` (default `devup.json`), `distDir` (default `df`) and
`sourceDirs` resolve against that root. Default numbering roots are the existing
`src` and `app` directories; use `sourceDirs` for other layouts. Malformed or
cyclic theme configuration stops initialization with a file-located error.
Optional file-number seeding failures emit one warning including the cause.

Build-time imported modules remain explicit side-effect imports in Bun's graph
so runtime watch mode can reload their importers. `Bun.build` takes CSS only
after extraction completes. Runtime imports synchronously publish changed CSS
to disk; identical revisions skip writes, without delaying read-after-import.

## MDX Ownership And Public API Limits

In `Bun.build`, selected MDX entries and selected files reached by literal
imports in modules Devup processes must go through Devup's own loader. A file
claimed first by another plugin is a located ownership error naming its target
and extension. Let Devup compile that extension, or remove it from
`mdxExtensions`. CSS finalization checks after `defer()`; an independent
`onEnd` check covers builds without stylesheet imports. Watch rebuilds that
execute these build callbacks receive the same check.

Bun does not expose loader chaining or another plugin's final module/resolve
target. A module loaded by another plugin under an extension Devup does not own
is invisible, including definitely used Devup exports. Such placeholders may
throw `Cannot run on the runtime` when rendered. Add the extension to
`mdxExtensions` and let Devup's project compiler load it instead of a separate
MDX plugin. Devup does not reject an indistinguishable harmless module or claim
to inspect another loader's final code.

Whole namespace/require-result uses and dynamic imports do not prove which
export is used. They remain opaque rather than being conservatively rejected.
Runtime helpers and unused imports are not compile-time-use violations.

The whole-build ownership guarantee does not apply to `Bun.plugin` runtime
registration: that API has no build `onEnd`/`defer` finalization point. Runtime
loading still compiles files Devup owns. An `onEnd` rejection can occur after
output files are written, even with `throw: false`; publish build artifacts only
after successful completion. Ownership diagnostics use the real importer
`:1:1` as an honest file-level fallback because this callback has no import
span; a structured CSS-load diagnostic may identify the virtual stylesheet.

## Custom Shorthands

To configure custom shorthands, preload a local module instead of the
zero-config entry:

```toml
# bunfig.toml
[test]
preload = ["./devup-ui.preload.ts"]
```

```ts
// devup-ui.preload.ts
import { register } from '@devup-ui/bun-plugin/register'

await register({
  shorthands: {
    insetX: ['left', 'right'],
    scrollMarginX: ['scrollMarginLeft', 'scrollMarginRight'],
  },
})
```

Custom shorthands are plugin configuration, not theme tokens. Every target
receives the same value, and target names may use camelCase or kebab-case. The
generated `df/theme.d.ts` provides type completion for component props,
responsive values, and selectors. Restart Bun after changing the option.
If `tsconfig.json` only includes your source directory, add `df/*.d.ts` to
`include`.

```tsx
<Box insetX={[0, null, 'auto']} _hover={{ insetX: 4 }} />
```

## Features

- Zero Config
- Zero FOUC (Flash of Unstyled Content)
- Zero Runtime
- Full Bun bundler integration
- TypeScript support with generated theme types
- CSS extraction at build time

## Theme Configuration

Create a `devup.json` file in your project root:

```json
{
  "theme": {
    "colors": {
      "default": {
        "text": "#000",
        "background": "#fff"
      },
      "dark": {
        "text": "#fff",
        "background": "#000"
      }
    }
  }
}
```

The plugin will generate `df/theme.d.ts` with TypeScript types for your theme.

## How It Works

The plugin transforms your JSX/TSX code at build time:

```tsx
// Before
<Box bg="red" color="$text" />

// After
<div className="d0 d1" />
```

Generated CSS:

```css
.d0 {
  background-color: red;
}
.d1 {
  color: var(--text);
}
```

## License

Apache-2.0
