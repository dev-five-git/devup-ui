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
