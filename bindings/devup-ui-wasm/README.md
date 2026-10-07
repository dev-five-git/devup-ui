# @devup-ui/wasm

The Devup UI compiler as a Node.js WebAssembly module. It is the engine the
build plugins (`@devup-ui/vite-plugin`, `next-plugin`, `webpack-plugin`,
`rsbuild-plugin`, `bun-plugin`) call; most projects never import it directly.

It is built from the Rust crates in `libs/` (`extractor`, `sheet`, `css`) with
`wasm-pack build --target nodejs`, so it loads with `require`/`import` in Node
and Bun, not in a browser.

## Build

```sh
cd bindings/devup-ui-wasm
bun run build   # removes pkg/, runs wasm-pack, writes pkg/package.json
```

`pkg/` is generated and ignored by git. The build starts from an empty `pkg/`,
so files of a removed export cannot linger there; `bun run verify:dist` in the
repository root fails if one does.

## API

Everything is exported from `pkg/index.js` (types in `pkg/index.d.ts`).

| Function                                                                                                          | Does                                                                                                                                                                                   |
| ----------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `codeExtract(filename, code, package, cssDir, singleCss, importMainCssInCode, importMainCssInCss, importAliases)` | Compiles one source file and returns an `Output`: the rewritten `code`, its `css`, `cssFile`, source `map`, the `dependencies` it read and `updatedBaseStyle`. Throws on a build error. |
| `codeExtractWithoutSourceMap(...)`                                                                                | The same without the source map.                                                                                                                                                       |
| `hasDevupUI(filename, code, package)`                                                                             | Whether the file imports Devup UI, so a plugin can skip the rest.                                                                                                                      |
| `getCss(fileNum, importMainCss)`                                                                                  | The stylesheet collected so far: the whole sheet for `null`, one file's for its number.                                                                                                |
| `registerTheme(theme)`, `registerShorthands(shorthands)`                                                          | Registers the `devup.json` theme and the custom shorthand props before extracting.                                                                                                     |
| `getThemeInterface(package, color, typography, length, shadows, theme)`                                           | The `.d.ts` text that types the registered theme.                                                                                                                                      |
| `getDefaultTheme()`                                                                                               | The theme variant that applies without `data-theme`.                                                                                                                                   |
| `setPrefix(prefix)`, `getPrefix()`                                                                                | Class name prefix.                                                                                                                                                                     |
| `setDebug(debug)`, `isDebug()`                                                                                    | Readable class names instead of base-37 ones, with repeated declarations kept (the tests run this way).                                                                                |
| `setModuleResolver(resolver)`                                                                                     | `(specifier, importer) => ({ path, code })`: how imports are resolved, so imported stylesheets are evaluated and imported constants inlined.                                           |
| `setAtomHoist(threshold)`, `importFileRoutes(routes)`                                                             | Moves atoms used by `threshold` routes or more into the shared sheet. Call before `codeExtract`.                                                                                       |
| `exportSheet`, `exportClassMap`, `exportFileMap`, `exportCanonicalMap` and the matching `import*` functions      | Save the collected state and load it into another process, as a bundler does between its server and client builds.                                                                     |

## State

The module keeps its state, the sheet, class maps, theme, prefix and debug flag,
in one global per process. Two consequences:

- Register the theme, shorthands, prefix and resolver before the first
  `codeExtract`, and extract the files of one build in one process.
- Tests that share a process share that state. The internal/testing-only
  `resetStateForTesting()` export (available in normal artifacts) clears the
  sheet, resolver, prefix/debug/hoist configuration, class/file/canonical/route
  maps, shorthands, theme-token registry and imported stylesheet caches. Call
  only at a quiescent test boundary, never in a resolver callback or between
  environments of a build that intentionally share state. Native caches reset
  on the calling thread.

Rust tests use hidden `css::test_state::TestStateGuard` and
`extractor::test_state::TestStateGuard` support compiled into dependencies so
dependent-crate tests can restore owned snapshots on normal exit, nesting and
unwind. Binding tests compose these with sheet/resolver snapshots. Keep a named
guard on its creating thread and retain `#[serial]`: guards clean up but do not
serialize concurrent tests. Cleanup does not run for panic=abort.

## Errors

A source the build cannot compile makes `codeExtract` throw an `Error`. Its
message names the call and says what the build needs, for example
``path/to/file.tsx:12:5: `css()` cannot use `window.name` at build time: its
values must be literals, theme tokens or constants`` (the location is included
whenever the problem has one). A `.css.ts` stylesheet that throws while it runs
reports that exception.

## License

Apache-2.0
