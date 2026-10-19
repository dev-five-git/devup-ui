# W72 Binding Experiment

This is a Windows x64 measurement harness, not a new published backend. Read
`W72-napi-report.md` for the decision, failed identity gates and measured limits.
The native crate is its own unpublished Cargo workspace and the published
WASM/plugin/core files are unchanged.

## Layout

- `native/`: synchronous napi-rs prototype, safe same-thread resolver and the
  JS Map/Set/undefined compatibility entrypoint. Load `index.cjs`, not `.node`.
- `harness/`: build-host scripts, a process-tree watchdog and result analysis.
- `data/`: fixed corpus manifest, summary/check records and compressed raw
  measurements. Compiled WASM/native binaries are intentionally not committed.
- `W72-napi-report.md`: measured numbers, determinism results and migration costs.

## Reproduce

Use latest stable Rust/Bun/Node/tools, but record actual resolved versions with
every rerun. The checked-in Cargo lock is the experimental measurement record,
not a production policy to remain on older tools. Updating dependencies changes
the experiment; retain the old evidence and regenerate all variants together.

Run from the checkout root. Use a private scratch directory outside the checkout
and copy the host scripts into it so their default output location is private:

```powershell
$root = (Get-Location).Path
$tmp = Join-Path $env:TEMP 'W72-local-repro'
if (-not (Test-Path -LiteralPath $env:TEMP)) { throw 'Missing temp parent' }
New-Item -ItemType Directory -Path $tmp -Force
Copy-Item benchmark/W72/harness/* $tmp
$env:W72_ROOT = $root
$env:W72_OUT = $tmp
$env:CARGO_BUILD_JOBS = '2'
$env:CI = $null
```

The watchdog rejects stack overrides and stops a current, creation-time-validated
process tree above 6 GiB of sampled RSS or private memory. Real builds additionally
require 7 GiB free physical memory at entry. Run only one compiling command at a
time. Its sampling interval is 500 ms plus WMI query time; it is not a kernel job
object limit. It must count the root process as well as current descendants.

1. Install workspace dependencies with the repository's Bun workflow, under the
   watchdog. Build shipping W1 with the exact current package command:

```powershell
& "$tmp/W72-watch.ps1" -Program bun -Arguments 'install' -Log "$tmp/W72-install.log"
& "$tmp/W72-watch.ps1" -Program bun -Arguments 'run build' -Directory "$root/bindings/devup-ui-wasm" -Log "$tmp/W72-build-W1.log"
bun "$tmp/W72-build.mjs" save-W1
```

2. Prepare private latest Node and the other variants. W2 uses the same optimizer
   as W1 but `-O3`; set `W72_WASM_OPT` to that executable on a different machine.
   The measured wasm-pack 0.15.0 cache held Binaryen 117. Do not silently compare
   different optimizer versions as if optimization level were the only change.

```powershell
bun "$tmp/W72-build.mjs" node-latest
& "$tmp/W72-watch.ps1" -Program bun -Arguments "$tmp/W72-build.mjs W2" -Log "$tmp/W72-build-W2.log"
& "$tmp/W72-watch.ps1" -Program bun -Arguments "$tmp/W72-build.mjs N1" -Log "$tmp/W72-build-N1.log"
& "$tmp/W72-watch.ps1" -Program bun -Arguments "$tmp/W72-build.mjs Nz" -Log "$tmp/W72-build-Nz.log"
& "$tmp/W72-watch.ps1" -Program bun -Arguments "$tmp/W72-build.mjs packages" -Log "$tmp/W72-build-packages.log"
```

W1 is opt-z/-Oz; W2 is opt-3/-O3; N1 is native opt-3; Nz is native opt-z. All use
LTO, one codegen unit, stripping and panic abort. Building native uses only the
new crate plus the same core path crates; no plugin import is permanently changed.

3. Freeze the complete input corpus, then measure in fresh interleaved processes.
   MDX compilation is outside timing. Use a new scratch directory when changing
   the corpus or source; do not mix old and new results.

```powershell
bun "$tmp/W72-corpus.mjs"
$env:W72_VARIANTS = 'W1,W2,N1,Nz'
foreach ($mode in @('identity','api','stack','timing','startup','reservation','headroom')) {
  & "$tmp/W72-watch.ps1" -Program bun -Arguments "$tmp/W72-run.mjs $mode" -Log "$tmp/W72-$mode.log"
  if ($LASTEXITCODE -ne 0) { throw "Failed: $mode" }
}
bun "$tmp/W72-headroom-verify.mjs"
```

`cold-state` resets the public engine state outside the timed call, not V8/OS
caches. Startup separately measures fresh processes. Timing includes input
transfer and output getters/free. The negative-scan boundary proxy is not a pure
FFI subtraction. Headroom uses isolated processes, not unsafe shared-addon
concurrent extraction or an already implemented async reservation path.

4. Run real builds one at a time through the watchdog with `-RealBuild`. Alternate
   W1/W2/N1 order and reverse it on the next round. Use at least three samples per
   app/variant and repeat suspicious results rather than removing slow samples:

```powershell
& "$tmp/W72-watch.ps1" -Program bun -Arguments "$tmp/W72-real-build.mjs landing W1 0" -RealBuild -Log "$tmp/W72-watch-real-landing-W1-0.log"
& "$tmp/W72-watch.ps1" -Program bun -Arguments "$tmp/W72-real-build.mjs vite W1 0" -RealBuild -Log "$tmp/W72-watch-real-vite-W1-0.log"
```

The runner preserves/restores the ignored generated binding alias and generated
landing inputs, inventories recognizable ignored caches before deleting them,
records actual CLI/API time, and hashes transformed code/maps/CSS. It never starts
a dev server or changes tracked plugin configuration. A forced kill cannot run
JS `finally`: inspect that the old tree is dead, then use `W72-recover.mjs` to
restore durable backups and the lock before another sample. Do not overwrite an
active transaction.

5. Analyze only after all samples finish:

```powershell
bun "$tmp/W72-analyze.mjs"
bun "$tmp/W72-real-summary.mjs"
bun "$tmp/W72-sanity.mjs"
bun "$tmp/W72-tables.mjs"
```

`W72-sanity.mjs` records production identity failures rather than normalizing them
away. The frozen-corpus gate passes while raw native cache portability and the
instrumented landing production byte-identity gate do not. A successful build
exit is not an identity pass. Local LSP/coverage gaps are documented in the report.

When replaying the recorded archive, set `W72_TIMING_ROUNDS=9` before the real
summary step. Primary performance rounds are 0-8; later rows are staged-harness
integration checks and are reported separately, without discarding primary slow
samples. A fresh run defaults to including all of its completed rounds.
