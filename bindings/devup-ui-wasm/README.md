# @devup-ui/wasm

Build-time CSS extraction bindings used by the Devup UI bundler plugins.

## Extraction boundaries

`codeExtract` generates a source map. `codeExtractWithoutSourceMap` skips map
generation, including when a module resolver is installed; its `map` getter is
`undefined`. Both functions return transformed source and watched dependencies.

`setModuleResolver` accepts a synchronous `(specifier, importer)` callback.
Return `{ path: string, code: string }` for a resolved module, or
`{ ignored: true }` for an explicitly ignored native alias. An ignored result
has no source/path and never reads `path`, `code` or `sourceType` getters.
`null` and `undefined` mean ordinary unresolved imports. Thrown exceptions, property-getter
and reflection failures, and missing/non-string fields instead fail extraction:

```text
/src/App.tsx:1:1: module resolver cannot use `./tokens` at build time: original cause
```

The importer includes transitive importers. The resolver contract does not
provide an import span, so this boundary reports `1:1` rather than guessing a
position from source text. The first resolver fault takes precedence over the
extractor's unresolved-module fallback and is checked **before** sheet updates.
The previous stylesheet contribution therefore survives a failed re-extraction.
Class/file numbering performed inside extraction is not rolled back.

Ignored imports are empty CommonJS modules, not renamed or virtual files. Missing
named values are `undefined`, defaults follow the existing CommonJS interop, and
namespace/require values are empty objects. No ignored file becomes a watched
dependency or is sent through the parser. The `ignored` discriminator must be
`true` or absent; malformed discriminators retain the same located resolver-fault
priority and previous-sheet preservation as malformed file results.

## Panic diagnostics

WASM initialization installs this binding's own `std::panic::set_hook`, with no
additional dependency or feature flag. It reports Rust's panic payload and
`file:line:column` location to `console.error` before the trap. A failing host
console does not replace the original panic. This is diagnostic reporting, not
panic recovery: with `panic=abort`, a panic still traps and the instance should
be discarded. Normal resolver/configuration failures use fallible JS errors.

Owned binding code does not unwrap user input or explicitly panic. Sheet mutex
poisoning is recovered, and no resolver callback is called while holding a sheet
lock or a thread-local resolver borrow. Panics originating in downstream crates
remain outside this binding's recovery contract.

## Verification

From the repository root:

```sh
cargo test -p devup-ui-wasm boundary_ -- --nocapture
```

After rebuilding `pkg` with this package's `bun run build`, run from the
repository root:

```sh
node bindings/devup-ui-wasm/boundary-regressions.mjs
node bindings/devup-ui-wasm/ignored-alias-regressions.mjs
```

This standalone runner loads the real generated JS and WASM, outside Bun's
coverage mocks. It exercises both extraction exports, map skipping with and
without resolution, unresolved null/undefined, valid resolved dependencies,
callback exceptions, getters, proxy reflection, malformed fields and hostile
exception objects. Every fault case verifies the previous sheet is unchanged.
Native tests cover every branch of the internal fault-precedence helper.

## Explicit Compiler Source Type

Both extraction functions accept an optional ninth argument after
`import_aliases`: `sourceType?: 'compiled-mdx'`. Use it for project-compiled MDX
under a custom extension, without changing the real filename:

```ts
const output = codeExtract(
  'src/page.mdown', compiledCode, '@devup-ui/react', 'df',
  true, false, false, {}, 'compiled-mdx',
)
```

The value means JavaScript with JSX, not TypeScript or raw Markdown. Omission
keeps filename-derived parsing, including compiled JavaScript/JSX for `.md` and
`.mdx`. Unknown extensions without an override fail with a located error even
when the source has no styling import. Invalid source-type values also fail
before stylesheet publication.

Resolved modules may return `{ path, code, sourceType: 'compiled-mdx' }`. The
child's own type reaches imported constants, reexports, stylesheet evaluation
and stripping caches; it never inherits the importing file's override. This
includes an ordinary TSX file importing a constant after JSX in compiled `.md`
or `.mdown`. Parser/source-type failures preserve the previous stylesheet.
Existing imported-value coordinate limitations remain owned by #757.
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
| `exportSheet`, `exportClassMap`, `exportFileMap`, `exportCanonicalMap`, `exportReservationState` and the matching `import*` functions | Save an owner's collected state, including the reservation phase and miss count, across synchronous engine resets. |
| `reserveClassSlots(rows, package, singleCss, importMainCssInCode, importMainCssInCss, importAliases)` | Reserve class/file counter slots without publishing CSS. Rows are `[filename, code, cssDir]` or `[filename, code, cssDir, 'compiled-mdx']`; compiled sources keep their real filenames. |
| `getUnreservedClassSlotCount()` | Count new live allocator keys after reservation, excluding repeated keys. |
| `getSuppressedReservationRowCount()` | Count failed rows suppressed by the latest reservation batch, independently of live misses. An empty or successful retry batch reports zero. |
| `resetBuildState()` | Reset build-scoped engine state before a fresh build or owner restore. |

## State

The module keeps its state, the sheet, class maps, theme, prefix and debug flag,
in one global per process. Integrations reset and restore their owned
snapshots at synchronous operation boundaries. Two consequences:

- Register the theme, shorthands, prefix and resolver before the first
  `codeExtract`, and extract the files of one build in one process.
- Tests that share a process share that state. The internal/testing-only
  `resetStateForTesting()` export (available in normal artifacts) clears the
  sheet, resolver, prefix/debug/hoist configuration, class/file/canonical/route
  maps, reservation phase/miss count, frozen atom plan, shorthands, the complete
  theme-token and Tailwind definition registries, and imported stylesheet caches. Call
  only at a quiescent test boundary, never in a resolver callback or between
  environments of a build that intentionally share state. Native caches reset
  on the calling thread. Opaque reservation snapshots must be restored with the
  matching sheet and allocator maps, not imported into another owner's maps.

`resetBuildState()` remains the separate production build boundary. It clears
allocator bookkeeping, maps/routes, atom threshold/plan, prefix, sheet and resolver,
but leaves CSS theme-key configuration, shorthands and debug for the next build's
configuration path to overwrite. It is not an alias for the full testing reset.

Scratch reservation failures are row-local: new class, variable, keyframe and
other shared allocator keys, and extraction-time file inserts, are rolled back
before the next row. The deterministic FILE inventory preseeded for the batch
is retained, including failed rows. A later successful retry appends slots without
renumbering existing keys. Suppression is captured in the opaque reservation
snapshot and cleared by `resetBuildState`; it never changes the live-miss query.
This rollback applies to scratch reservation only, not failed real transforms.

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

After rebuilding the bindings, run the real project-compiler/WASM matrix:

```sh
node bindings/devup-ui-wasm/source-type-regressions.mjs
```

The same JS runner also uses the installed `@mdx-js/mdx` compiler, resolved through
`@mdx-js/loader` in `apps/landing` (no added dependency). Both default `_jsx`
output and `jsx: true` output are extracted under real `.mdx` and `.md` filenames.
It checks CSS, the relevance gate, source-map filename identity and rejection of
raw Markdown. Compiled MDX is parsed as JSX JavaScript, not TSX. Its source-map
coordinates refer to compiler output; bundler integrations must compose input
maps to recover Markdown positions, or identify diagnostics as compiled MDX.

Focused native MDX verification:

```sh
cargo test -p extractor mdx_ -- --nocapture
```

Apache-2.0
