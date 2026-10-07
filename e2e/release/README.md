# Release compatibility gate

`bun run test:release` packs the built Devup UI packages, installs those tarballs
into isolated consumer projects, builds production artifacts, and drives their
actual output in Chromium, Firefox and WebKit. It never imports plugin internals
or asserts generated class names, stylesheet names, or chunk layouts.

Build the repository packages first with `bun run build`. The gate reuses the
tool versions already installed by the repository lockfile, rather than adding
another version policy for the fixtures. Each consumer starts without generated
compiler state or build output and exercises both per-file and single CSS modes.

The standalone Webpack consumer calls the public Webpack compiler API directly,
not Next's build pipeline, and uses the CJS Devup UI plugin entry. Its Webpack
engine is the version vendored in the already-installed Next package (5.98.0 on
the approved base); JSX uses the repository's existing TypeScript 6 compiler.
This avoids adding an external dependency. The vendored-engine result is not a
claim about an independently installed or different Webpack version.
Next's vendored engine references a standalone default-minifier module that its
package does not ship. This consumer uses the public production configuration
`optimization.minimize: false`; it verifies compilation and CSS delivery, not
minifier compatibility. The missing default-minifier module is retained as a
toolchain finding rather than attributed to Devup UI.

The browser checks layout primitives, static spacing/color, an imported styling
module, responsive rules, hover selectors, extended theme tokens, and a dynamic
value delivered through the compiler's CSS variable path. A browser error or
missing style fails the gate even if the bundler exited successfully.

Each consumer also rebuilds after same-length style and extended-theme edits and
removing a style prop, then rechecks the browser output. The Vite consumer includes
an unsafe runtime style input that must fail its production build with a source
location. These are production rebuild checks, not a claim of dev-server HMR
coverage.

All builds and browser runs are sequential. A foreground watchdog samples the
private memory of the complete command process tree every second, stops that
tree above 6 GiB or after ten minutes, and treats a stop as a finding. Servers use
ephemeral ports and close in `finally`; remaining owned child processes are
terminated before the command returns. CI applies a 30-minute job timeout too.

`release-results.json` records failures, command output, duration, and peak
observed private memory. CI uploads it even on a failed fixture. A known plugin
bug remains a failing gate until the owning PR fixes it; do not add fixture
workarounds, expected failures, or skipped assertions.

The separately guarded package build writes `release-package-build.json` even
when it fails before consumer setup. CI first runs the process-memory tests:
an exited Linux process (`ENOENT` or `ESRCH` during a proc read) contributes zero,
while permission failures remain errors rather than silently undercounting a
live process. The 6 GiB cap is unchanged.

For a local focused run, set `RELEASE_TARGETS` to a comma-separated target list
and `RELEASE_BROWSERS` to a comma-separated browser list. CI leaves both unset
and always runs the full matrix. `RELEASE_FIXTURE_ROOT` selects the parent of the
run-unique temporary directory; supervised workers must set their private
worker directory. Failed consumer directories are retained for investigation;
successful runs remove only their own temporary root.

The Bun 100% unit metric covers handwritten package source, including the Bun
registration hooks. Generated `pkg`/`dist` code is not counted by that metric:
the real WASM ABI suite, packed consumers, and browser results are separate
behavioral evidence, not a claim of 100% generated-WASM coverage.
