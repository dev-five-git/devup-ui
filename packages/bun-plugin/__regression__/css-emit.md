# Bun CSS emission regression

The source at `6f9079ec` already resolves injected CSS imports to a path-free
virtual module (`src/css-id.ts`) and loads it as an empty JavaScript module.
That fixes Bun runtime loading and cross-worktree transpiler-cache isolation,
but `writeDataFiles()` only creates the CSS directory and `loadSourceFile()`
discards extracted CSS. No stylesheet is written.

The other plugins have different bundler lifecycles:

- Next initializes `devup-ui.css` with `getCss(null, false)` and refreshes CSS
  through its loaders/coordinator.
- Webpack writes initial CSS in watch mode, writes extracted CSS in its source
  loader, serves CSS through its CSS loader, and writes the final base sheet
  after a successful build.
- Vite creates the directory before its initial base stylesheet, materializes
  CSS imports during transformation, and serves/finalizes CSS from `getCss`.
- Rsbuild materializes stylesheet imports during transformation and serves
  their contents through its CSS transform.

Bun uses `singleCss = true`, so its artifact is `df/devup-ui/devup-ui.css`.
Initialize it after directory creation with actual theme CSS, then refresh it
from `getCss(null, false)` after extraction. A placeholder alone would discard
the styles, since Bun's virtual runtime module intentionally contains no CSS.
Keep virtual resolution for cache isolation. No shared utility is needed for
this small change: the other plugins' emission timing and loaders differ.

Run `bun run build`, then
`bun run --filter @devup-ui/bun-plugin test:regression`.
The CSS regression runs isolated Bun processes in temporary directories without
`df`, using the built plugin and real WASM. It checks initial emission with and
without a theme, parallel extraction of eight distinct styles, and CSS imports.
It disables the transpiler cache for these cold-start cases; the existing
worktree-isolation regression continues to exercise the shared cache.

## Verification (Windows, Bun 1.4.1)

Before implementation:

- Initial `bun test`: 0 pass, 66 fail/errors because workspace build artifacts
  were missing. Initial `bun run lint`: Rust checks passed; ESLint could not
  load the unbuilt local ESLint plugin.
- Initial `bun run test`: Rust tests passed, coverage 98.21% (8572/8728 lines);
  the Bun phase failed because build artifacts were still missing.
- `bun run build`: passed.
- Prepared `bun test`: 5180 pass, 0 fail; 87 snapshots; 100% function/line
  coverage under the existing repository configuration.
- Prepared `bun run lint`: passed (format, Clippy, ESLint).
- Regression RED, before production edits: 2 existing tests passed; all 3 new
  cases failed because the stylesheet did not exist (`false` / `ENOENT`).

After implementation:

- `bun run --filter @devup-ui/bun-plugin build`: passed.
- `bun run --filter @devup-ui/bun-plugin test:regression`: 5 pass, 0 fail,
  including the unchanged preload and worktree-cache regressions.
- `bun run test`: passed. Rust coverage remains 98.21% (8572/8728 lines,
  +0.00%); Bun remains 5180 pass, 0 fail, 87 snapshots and 100% function/line
  coverage. No coverage configuration or existing assertions were changed.
- Running Tarpaulin and Clippy concurrently caused transient missing `target`
  fingerprint files on Windows. Validation commands were rerun sequentially.
- Sequential `bun run lint`: passed, matching the prepared baseline.
