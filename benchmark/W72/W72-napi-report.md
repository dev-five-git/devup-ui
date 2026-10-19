# W72: WASM Versus napi-rs, Same Devup UI Core

2026-10-19. Experiment only, based on `f014de09f2f3b5b80a40edbb16c497bf7bba6908` on `experiment/napi-binding`. No published package, core extraction algorithm, plugin configuration, runtime styling behavior, or CI workflow is changed. No PR is requested or created.

## Decision

**Do not switch the published binding yet.** Native N1 improves fresh-process startup, callback-heavy production extraction and reservation, but does not consistently beat speed-optimized WASM on warm extraction. In particular, W2 beats N1 on the 5000-element warm call. Whole-app timings remain dominated by work outside Devup and shared-machine variance. Native also requires a new distribution/support matrix and a deliberate cache-identity decision. Most importantly, the instrumented landing production path does not pass byte identity even between separate W1 builds; passing deterministic frozen inputs is not sufficient migration signoff.

**Evaluate W2 independently, not as a prerequisite to napi-rs.** It materially improves warmed extraction without an API or distribution migration, but nearly doubles the artifact and is slower to initialize/JIT on this machine. Keep W1 as the default until a measured workload justifies accepting the size/startup step. The #746 WASM size gate would need one explicitly accepted baseline change; this experiment does not change that gate.

## Method And Provenance

- Machine: Windows 10.0.26300 x64, AMD Ryzen 9 9900X, 12 cores/24 logical processors, 31.09 GiB physical memory. Other workers were active; this is not an isolated-machine claim.
- Latest versions rechecked from crates.io/npm/GitHub at start: Rust stable 1.99.1, Bun 1.4.3, Node 26.12.0, napi 3.14.2, napi-derive 3.6.12, napi-build 2.6.0, `@napi-rs/cli` 3.10.8, `@napi-rs/wasm-runtime` 1.2.5. The shared Node was 24.19.0, so real measurements use a private latest Node 26.12.0 executable, not a shared downgrade/upgrade. CLI/WASM-runtime versions inform migration costs; neither is needed to load this directly built native experiment.
- W1: the current `bun run build` in `bindings/devup-ui-wasm`, `wasm-pack 0.15.0 --target nodejs`, workspace release `opt-level=z`, LTO, one codegen unit, strip, panic abort, metadata `wasm-opt -Oz`. Its generated module is snapshotted before other builds. This is the current source built exactly by the shipping command, not a claim to have downloaded the released npm tarball.
- W2: same source and lockfile, `CARGO_PROFILE_RELEASE_OPT_LEVEL=3`, wasm-pack `--no-opt`, then the same wasm-opt executable with `-O3` and the same feature switches. wasm-pack's cached/bundled optimizer reports Binaryen 117; it is intentionally the shipping tool for W1/W2 isolation, not an independently upgraded Binaryen experiment.
- N1: a separate unpublished Rust workspace in `benchmark/W72/native`, latest napi-rs, the same `css`, `extractor`, and `sheet` path dependencies, opt-level 3/LTO/one unit/strip/panic abort. Nz changes only opt-level to z. The existing core's native mutexes and wasm thread-locals are retained, not normalized away.
- Corpus: all 138 landing JS/TS sources, all 47 component JS/TS sources, all 96 landing MDX sources compiled once to JSX outside timed regions, the eight largest raw-import extractor fixtures, and a 5000-styled-element synthetic file. Total 290 inputs / 1,366,580 UTF-8 bytes. Six largest Rust fixtures depend on test-only imported contexts; their identical located errors are retained in identity results and excluded from successful-call latency, not silently counted as fast extraction. Raw sources, origins and hashes are in the manifest.
- Microbenchmarks: 3 alternating fresh processes per variant, 20 repetitions per input/mode/process; source-map and no-map paths; state-cold reset outside timed call versus warmed engine; getters and `free()` inside timed extraction. State export/import includes JSON.parse and actual adapter conversion. Startup uses 30 alternating fresh processes per variant. Percentiles use nearest-rank.
- Real builds use a temporary ignored `pkg/index.js` alias, restored byte-identically after each sample. Actual Vite/Vinext CLIs run on the private latest Node. Each cold run deletes only recognizable, ignored local app output/cache directories. Filesystem/OS caches are not flushed. API observations hash code/maps/CSS without replacing return values; their extra getter/hash overhead is separately recorded.
- The primary production timing set is nine alternating rounds per app/binding (54 successful builds, repetitions 0-8). Later repetition-9-and-up runs only validate the staged/refactored host harness; their raw records remain available but are not mixed into the performance tables. Archive replay sets `W72_TIMING_ROUNDS=9` explicitly.
- One compiling command at a time, `CARGO_BUILD_JOBS=2`, CI unset, no `RUST_MIN_STACK` or linker/Node stack overrides. Every build/test is under a 6 GiB RSS/private-memory watchdog; real runs require at least 7 GiB free at start. RSS is a sampled process-tree peak (500 ms plus WMI query time), not a kernel hard limit or an exact instantaneous maximum. Per-process Node peak RSS is also recorded.

## Extraction And State Calls

All latency pairs are median / p95 in milliseconds; nearest-rank percentiles. Extraction columns select the source-map-enabled path; no-map results remain in the raw summary. Corpus sums include both successful fixture inputs and the 96 precompiled MDX inputs, not Markdown compilation or the six imported-context negative fixtures.

| Variant | Corpus Cold State | Corpus Warm | 5000 Elements Warm | getCss | Sheet Export | Sheet Round Trip |
|---|---:|---:|---:|---:|---:|---:|
| W1 | 168.489 / 192.634 | 112.630 / 124.342 | 55.285 / 63.507 | 1.744 / 2.818 | 0.698 / 1.046 | 5.842 / 8.486 |
| W2 | 108.055 / 123.711 | 72.197 / 81.385 | 34.352 / 40.533 | 1.416 / 2.532 | 0.407 / 0.712 | 5.207 / 7.823 |
| N1 | 103.396 / 123.225 | 74.443 / 87.796 | 40.809 / 48.982 | 1.598 / 2.992 | 0.308 / 0.596 | 12.949 / 18.657 |
| Nz | 167.854 / 187.317 | 121.037 / 134.273 | 68.826 / 75.564 | 2.425 / 5.668 | 0.922 / 1.552 | 15.487 / 25.313 |

## Fresh Process Startup

30 interleaved fresh processes per variant, filesystem caches left intact. Import includes require/load; initialization adds reset, debug/theme/shorthand registration. First extraction is the synthetic 5000-element file including getters/free. RSS is the maximum pooled across 30 startup plus 3 timing processes per variant, not startup alone. Bytes count only the raw WASM/native binary, excluding loader/glue and compressed/npm distribution costs.

| Variant | Import Median / p95 ms | Import + Init Median / p95 ms | First Extract Median / p95 ms | Max Pooled RSS MiB | Binary Bytes |
|---|---:|---:|---:|---:|---:|
| W1 | 12.570 / 26.101 | 20.382 / 32.703 | 128.780 / 169.106 | 118.891 | 5,803,344 |
| W2 | 19.445 / 31.385 | 28.514 / 42.272 | 155.943 / 177.305 | 151.906 | 10,840,165 |
| N1 | 3.977 / 17.101 | 4.840 / 18.166 | 45.969 / 54.495 | 157.020 | 12,628,480 |
| Nz | 4.270 / 17.718 | 5.059 / 18.516 | 71.748 / 89.853 | 156.988 | 6,875,648 |

## Production Builds

Alternating cold builds per app/variant; each odd round reverses W1-W2-N1. Individual samples are retained rather than dropping slow runs. Wall time includes the landing generation step and actual CLI, not inventory/preflight/cleanup. Inside Devup is summed outermost synchronous binding API time, excluding separately measured observation work and binding import.

| App | Variant | Individual Wall ms | Median Wall ms | CV | Median Inside ms | Median Inside Share | Max Tree RSS MiB |
|---|---|---|---:|---:|---:|---:|---:|
| landing | W1 | 12910.886, 13494.891, 21463.824, 12815.573, 12553.672, 14796.903, 22920.041, 15077.967, 15577.964 | 14796.903 | 23.0% | 351.715 | 2.45% | 1190.840 |
| landing | W2 | 12478.019, 12298.678, 14063.129, 16098.312, 13952.599, 14310.442, 13996.736, 13640.865, 13616.707 | 13952.599 | 7.5% | 294.622 | 2.28% | 1176.883 |
| landing | N1 | 12700.809, 12581.841, 14063.185, 33509.008, 13404.280, 12939.397, 12773.431, 12685.970, 14155.601 | 12939.397 | 41.6% | 140.520 | 1.07% | 1116.535 |
| vite | W1 | 582.914, 325.875, 296.765, 397.844, 339.606, 1981.335, 672.136, 335.075, 321.565 | 339.606 | 87.3% | 25.098 | 7.55% | 189.480 |
| vite | W2 | 378.277, 395.600, 323.933, 399.004, 375.660, 339.369, 412.707, 357.103, 323.509 | 375.660 | 8.5% | 43.868 | 12.09% | 181.434 |
| vite | N1 | 286.759, 328.796, 317.315, 679.776, 326.261, 313.174, 308.202, 263.983, 315.639 | 315.639 | 34.0% | 2.035 | 0.64% | 164.668 |

Peak tree RSS uses the root-inclusive v2 watchdog samples (at least three per app/variant), after correcting Windows creation-time precision and stale-parent PID attribution. Earlier timing samples are retained, but their older sampler RSS is not used for this column.

Shared-machine Windows CPU-load snapshots range 10-68%; starting free memory 7.53-9.73 GiB. Instrumentation getter/hash overhead ranges 0.530-30.495 ms per build.

## Production Identity Gate

Each range is the number of differing diagnostic records/files against the first W1 build, including the baseline zero. Per-record keys do not distinguish repeated environment/options and are not alone a final-output proof. The last two columns independently compare multisets of actual dist CSS/JS payload hashes, ignoring filename-only renaming; they confirm landing differs even W1-versus-W1. The Vite demo is stable.

| App | Variant | DF CSS Different | Code Records Different | Maps Different | CSS Filename Different | Dependencies Different | Final CSS Equal Runs | Final JS Equal Runs |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| landing | W1 | 0-10 | 0-54 | 0-0 | 0-9 | 0-0 | 1/9 | 1/9 |
| landing | W2 | 7-10 | 7-54 | 0-0 | 7-9 | 0-0 | 0/9 | 0/9 |
| landing | N1 | 6-10 | 6-54 | 0-0 | 6-9 | 0-0 | 0/9 | 0/9 |
| vite | W1 | 0-0 | 0-0 | 0-0 | 0-0 | 0-0 | 9/9 | 9/9 |
| vite | W2 | 0-0 | 0-0 | 0-0 | 0-0 | 0-0 | 9/9 | 9/9 |
| vite | N1 | 0-0 | 0-0 | 0-0 | 0-0 | 0-0 | 9/9 | 9/9 |

## Boundary Proxy And Reservation

`hasDevupUI` on a 1,000,000-byte ASCII non-import string measures inbound string conversion plus the real fast-negative scanner, not pure transfer. Native includes its actual N-API entry, WASM its actual generated glue. Reservation is the real no-callback sequential pre-pass over all physical corpus rows.

| Variant | 1 MB Negative Scan Median / p95 ms | Reservation Median / p95 ms |
|---|---:|---:|
| W1 | 1.325 / 2.138 | 269.865 / 306.199 |
| W2 | 1.350 / 1.938 | 347.377 / 407.820 |
| N1 | 0.044 / 0.065 | 78.673 / 94.193 |
| Nz | 0.086 / 0.114 | 119.535 / 129.684 |

## Isolated Process Headroom

This is not an async binding or a usable parallel reservation implementation: per-input state is reset and no-callback/no-map extraction plus output materialization is split across one or two isolated processes. A production plan still needs the authoritative sequential reservation and ordered sheet commit.

| Processes | Individual Wall ms | Median Wall ms |
|---|---|---:|
| 1 | 706.783, 651.454, 684.211, 727.118, 708.316, 798.335 | 706.783 |
| 2 | 438.457, 444.261, 428.334, 458.916, 491.570, 511.633 | 444.261 |


## Where The Time Goes

The speed-optimization control is essential: compare W1 with W2 before attributing a native-versus-WASM gap to FFI. N1-versus-Nz likewise shows substantial native code-generation cost. Optimizing the binding alone does not remove parsing, evaluation, allocation, sorting, CSS creation, cloned output getters, or the core's different native synchronization path.

The large-string negative scan is a **boundary proxy**, not a subtraction-based measurement of pure transfer: it includes the core's actual scanner. The small N1 proxy cost compared with its tens-of-milliseconds synthetic extraction shows that substantial time remains in real extraction/sheet work. WASM's generated UTF-8 encoding/copying is material for large inputs, but the warm W2 synthetic win proves that transfer cost alone does not determine the winner. No exact transfer-versus-compute percentage is claimed without an instrumented core or a profiler.

Native loading avoids WASM compilation/tiering and theme initialization is cheap. W2's larger binary improves warmed computation but makes fresh-process import/first-call behavior worse here. The first ever N1 load before the repeated startup series took about 251 ms; its cause was not isolated. The table is explicitly repeated fresh processes with warm OS caches, not a disk-cold guarantee.

The state-call series follows repeated export/import. As the current API intentionally omits theme from its sheet snapshot, most `getCss` samples in that series reflect restored rule storage without the theme block. The separate API-equivalence scenario re-registers theme before comparing CSS. N1's slower round trip includes the conservative JSON/Map compatibility adapter and intermediate serde value, not an intrinsic napi-rs lower bound; a production typed-deserializer optimization needs its own equivalent-contract measurement.

The real build share limits the possible end-to-end gain: even eliminating the measured synchronous Devup calls entirely cannot remove the rest of the build. Summed API time excludes binding import, getter observations and plugin-side filesystem/graph work; it is not all Devup-related work and is not a CPU profile. Treat overlapping total-build ranges as noise rather than declaring a universal build winner. The tiny Vite app disproportionately exposes cold WASM tiering.

## Identity And Default Stack

**Emitted code, CSS, source maps, dependencies and class names match byte-for-byte for all successful corpus inputs across W1/W2/N1/Nz, in single and per-file CSS modes and with/without maps.** W1/W2 also have identical exported snapshots. Each native variant repeats identically across three separate processes. The common negative fixtures report identical errors. These are comparisons of actual bytes, not formatted ASTs or CSS normalization.

The additional API scenario covers resolver callbacks/dependencies and caught resolver throws, theme/default theme/interface generation, JS Map shorthand registration and Set routes, prefix/debug, file seeding/canonical buckets, atom hoist and keyframes, state round trips, malformed reservation packet nonmutation, reverse-order transforms after reservation and zero unreserved slots, reset, and the existing 32-dependent-capture source shape in ESM/CJS. Its outputs match W1/W2/N1 on both Node and latest Bun. Theme is deliberately re-registered after importing a sheet: `StyleSheet.theme` is skipped by the existing cache serialization, not a new native behavior.

**Raw native cache bytes are not WASM-identical.** Every mode has a different `exportSheet()` string because `PropertyMap` serializes unsorted FxHashSet arrays (`libs/sheet/src/lib.rs:255`, `:287`, `:310`). Its hash uses target-width-dependent FxHasher state, so wasm32/x64 bucket traversal differs, even though CSS emission sorts the rules. `exportClassMap()` also differs at keyframe identity keys: `KeyframeIdentity` hashes map/vector `usize` lengths and enum-bearing fields with `DefaultHasher` (`libs/extractor/src/extract_style/extract_keyframes.rs:20`). The slot ordinals and emitted names match in these runs, but the decimal `k-...` cache keys do not. Example StarButton key: WASM `k-15165798619612131214`, x64 native `k-18176125330098819799`. Mixed-backend persistent caches are not approved; use backend/target/compiler cache isolation or intentionally migrate to a fixed-width structural key after a separately reviewed compatibility decision.

**Production landing identity fails the stronger gate.** The Vite demo's CSS and transformed code/maps remain identical, but landing's collected DF CSS and some transform records differ across repeated W1 builds as well as W2/N1 builds. A separate comparison of actual dist CSS/JS payload-hash multisets (not merely filenames or intermediate caches) also differs: all 210 CSS payloads per landing build are considered, and only the baseline matches itself, while the Vite demo's one CSS payload and JS payload set match in all 27 primary builds. All source hashes and observed per-record field differences are retained; repeated transform records are diagnostics because their keys do not distinguish environment/options. Since W1 differs from itself, this cannot be assigned to native RandomState solely from the cross-binding comparison. The current Vite plugin lacks the shared physical-input class reservation implemented by the separately pending plugin work; asynchronous transform/encounter ordering is a plausible seam, not a proven root cause from this experiment. Read-only hash/getter instrumentation can perturb scheduling and has not been ruled out as a trigger. The finding was escalated to the coordinator, not patched into published plugin/core behavior. A follow-up must replay captured production inputs with matching reservation state and reproduce uninstrumented builds before either binding/optimization migration is approved. Do not call the production identity requirement passed.

No output-relevant unsorted std `RandomState` map iteration was found in the inspected paths. Do not infer that from Rust's hasher defaults alone. The concrete inventory is:

| Site | Use / Ordering |
|---|---|
| WASM binding `lib.rs:258`, `:283`, `:310` and native `state.rs` | Class, file and canonical exports explicitly collect into BTreeMaps; class export sorts both levels. |
| WASM binding `lib.rs:589`; extractor `lib.rs:342`; `import_alias_visit.rs:361` | Alias-map iteration converts lookup values or computes a pure any-match boolean; source rewriting follows authored/source-position order. |
| `extractor/emotion_namespace/reads.rs:8` | HashMap bindings are collected and sorted by SymbolId before lowering/fresh-name generation. Namespace sets otherwise serve membership/collision checks. |
| `css/atom_hoist.rs:51` and `file_routes.rs:94` | Route HashSets/HashMaps are unioned; only membership/cardinality escapes, with the frozen eligibility plan a BTreeSet. No slots allocated by route iteration. |
| `css/class_map.rs`, `file_map.rs`, `slot_reservation.rs` | Lookups, insertion/length and scratch snapshot/restore. File seeding sorts/deduplicates paths; first-encounter behavior still matters without seeding. |
| `sheet/theme.rs:1028` | Default-token HashMap is a lookup cache; ordered BTreeMaps/sorted variant lists drive actual emission. |
| `sheet/lib.rs:612`, `:1005`, `:1087`, `:1187` | Fx-backed rules are sorted for allocation/CSS; raw serialized property-set arrays are not. Raw snapshot bytes remain a separate failing portability gate. |
| `extract_style/extract_keyframes.rs:20` | Non-randomized DefaultHasher stream is still platform/compiler-sensitive, as actually observed above. |

This is an inventory of the inspected engine/binding paths, not a proof about arbitrary future dependencies or untested input programs.

Nested authored JSX at depths 16/32/64/128/256/512 succeeds on every variant in separate Node processes with their default stacks. The 32-dependent-capture regression source compiles through each binding in both module dialects. No stack size is raised. This establishes only the tested depths and surfaces, not unbounded nesting, equivalence to libuv/Tokio worker stacks, or Miri/FFI soundness. Native panic-abort/stack overflow can terminate the build host; a WASM failure is normally an instance trap. Those failure contracts need explicit migration testing.

## N2 Is Headroom, Not A Shippable Async Path

The process-isolated measurement compares the same candidate computations; sorted candidate hashes match in all six 1-process versus 2-process pairs. It does not run the process-global engine concurrently inside one addon and does not rename anything in production. Parallel candidate output is discarded. The real authoritative reservation remains sequential in supplied physical-input order, including sheet-dependent allocation, followed by sequential live replay/emission. These costs, IPC and state distribution cannot be wished away or subtracted as if a parallel implementation already existed.

Latest napi-rs supports Promise-returning async functions and AsyncTask/libuv compute, and can target WASM. That capability does **not** make this core safe to call concurrently: native class/file maps, sheet, theme/token registries and atom plan are separate process globals, while resolver/evaluator state is thread-local. Individual mutexes are not an extraction transaction. Start with per-build isolated engine contexts or process isolation, a deterministic reservation/commit protocol, and zero missed slots before considering a production pool.

The current Rust resolver is a synchronous `Fn(&str, &str) -> Option<ResolvedModule>` calling JS inline. Off-thread extraction needs either an immutable pre-resolved graph/native resolver (for example `oxc_resolver`, with explicit alias/conditions parity) or a result-aware ThreadsafeFunction round trip. Do not move Env/FunctionRef/JS values to a foreign thread, block the JS thread waiting for work that needs that thread, or mistake TSFN queue-blocking mode for a synchronous callback-result contract. Preserve caught-throw-as-miss semantics, bounded queues, error handling, cancellation and teardown. Async spelling alone does not shorten CPU work.

## Switching Cost And Migration Outline

N1's entrypoint is `index.cjs`, not the bare `.node`: it adapts ordinary objects, Maps, Sets and absent Option values before napi's JSON boundary. That adapter's real cost is included. The tested 27-function API, optional output getters and normal free/dispose usage match, but exotic iterables/prototypes/symbols, non-finite numbers/BigInt, post-free traps and malformed-input error wording are not full serde-wasm-bindgen parity claims. A production drop-in must specify and test these boundaries rather than treating this prototype as a complete replacement.

1. Keep the JS API contract and all build-time-only behavior. Factor the WASM/native boundary over shared tested extraction/output/state code rather than maintaining two long-term copies. This experiment intentionally copies boundary logic to avoid changing published behavior.
2. Resolve the cache issue before rollout. Either explicitly segregate caches by backend/target/compiler and invalidate on switches, or adopt canonical sorted snapshot arrays plus portable structural/fixed-width keyframe identities with golden byte tests. The latter is an observable cache/naming contract change, not an experiment-only cleanup. Preserve all existing emitted names; do not silently accept newly allocated classes from old mixed caches.
3. Publish a loader package plus optional platform packages/prebuilt binaries. A baseline matrix is Windows x64/arm64 MSVC; macOS x64/arm64; Linux x64/arm64 GNU and musl, extending only to actually supported targets. Test Node latest and supported Node majors plus Bun latest per supported target; this run verifies only Windows x64 Node 26/Bun 1.4.3. Account for libc/CPU-baseline selection, npm optional-dependency installation behavior, checksums/provenance, symbols and failure messages. No postinstall compiler requirement for ordinary consumers.
4. Add a separately built `wasm32-wasip1-threads` fallback for WebContainers/browser-like build hosts, or retain an adapted existing wasm-bindgen backend. The napi WASI target is not this wasm-bindgen binary: generated loaders, WASM, worker files, `@napi-rs/wasm-runtime` and compatible emnapi runtime/core must travel together. Validate shared memory/workers, filesystem semantics, forced-fallback selection and WebContainer behavior. Bun native support is best-effort upstream; Bun WASI gaps mean native Bun success here is not a fallback guarantee. No fallback binary or async addon is built/measured in W72.
5. Cross-compile/release CI needs native platform/ABI jobs plus the WASI job, a loader selection test that proves which backend ran, cross-process determinism/state interoperability tests, default-stack crashes in isolated processes, and Node/Bun import/build probes. Use current `*-latest`, stable Rust and latest tools rather than freezing this measurement's tool versions. Run the same fixtures through every shipped backend before packaging.
6. Update each actual affected package's release metadata with **Patch** changepacks: binding/loader and new platform/fallback packages, plus plugin-utils and every Bun/Next/Rsbuild/Vite/Webpack package whose imports/distribution change. This experiment changes none of those packages and adds no production changepack.
7. Roll out sync native first only if representative build results justify it; keep a verified WASM fallback and explicit cache separation. Async/pooling is a separate state/resolver architecture project, not a flag on this prototype. Preserve zero styling runtime in browser bundles throughout.

### Reproduction And Evidence

The experiment harness, native crate, corpus manifest, summary JSON, raw compressed measurements and this report live under `benchmark/W72`. Run from the checkout root with `W72_OUT` pointing to a private scratch directory; stage the harness there or set paths as described in the harness README. Use the supplied capped PowerShell runner for builds/tests. The evidence archive retains full output snapshots, per-call samples, per-build API/output hashes, environment and watchdog records without compiled binaries. Final commit/push status is delivery-specific, as recorded below.

The exploratory harness initially rejected Vinext's generated `.next/types` directory as an unknown cache shape; that guard was corrected after inspecting the actual output. One later preflight was falsely stopped by a watchdog's stale-PID ancestry tracking before alias installation; the corrected watchdog rebuilds the current root tree and filters creation times. No benchmark timing from those rejected attempts is included. All final real samples use the corrected watchdog. Aliases/generated input bytes are restored, and no build servers remain.

Root-inclusive RSS precision was subsequently checked with a 256 MiB allocation smoke run, and a final three-round primary phase per app/binding uses that verified sampler. The RSS column uses only those root-inclusive v2 records; older timing observations remain in the latency distribution. The staged runner also audits for lingering Node/Bun children without force-killing an uncertain PID, avoiding stale-parent ownership assumptions. This post-measurement host hygiene does not change the generated alias template or timed compiler calls.

Official references checked for migration costs: [napi async functions](https://napi.rs/docs/concepts/async-fn), [AsyncTask](https://napi.rs/docs/concepts/async-task), [ThreadsafeFunction](https://napi.rs/docs/concepts/threadsafe-function), [WebAssembly target/runtime](https://napi.rs/docs/concepts/webassembly), [support compatibility](https://napi.rs/docs/more/support-compatibility), and [napi 3.14.2 rustdoc](https://docs.rs/napi/3.14.2/napi/). Context7 was attempted but its quota was exhausted, so official rustdoc/site and live registry records were used instead. W72's values are local observations, not numbers supplied by those references.

## Validation At This Checkpoint

| Gate | Observed Result |
|---|---|
| Shipping W1, speed W2, native N1 and Nz builds | All exit 0; serial two-job builds, default stack |
| Native standalone strict clippy, all targets | Exit 0 after metadata-only lint placement fix; napi duplicates function attributes onto generated shims, so documented expectations are scoped to the three existing bridge modules |
| Native libtest | Exit 0, zero Rust unit tests; actual normal/API/error/reservation behavior is tested by the Node/Bun harness, not invented unit coverage |
| Workspace cargo fmt / strict clippy / cargo test | Exit 0; no pending `.snap.new` files |
| Root lint | Exit 0 after formatting only the new native JS entrypoint; 20 existing React recommendation warnings remain, with no added warning |
| Full Bun suite | **6340 pass, 1 baseline failure**, 100.00% functions and lines; the failing `changepack-paths.test.ts` rejects `apps/landing/package.json` in the existing transient-props changepack at f014de09 |
| Windows LLVM tarpaulin | Exit 0 but **97.88%**, 19,589/20,014 lines; the mandatory 100% target is not achieved locally and is not claimed. No published Rust source changed in this experiment |
| LSP diagnostics | Unavailable: TypeScript server not installed (previously declined); Rust daemon requests timed out. Compiler/clippy/test evidence is supplied instead, not fabricated LSP-clean status |
| Frozen corpus / API / process repeat / default stack | Passed as detailed above; raw-cache portability and production landing byte-identity gates explicitly fail |
| Publication | Not yet committed/pushed at this checkpoint: coordinator question `msg_f489fc9fee8e` asks whether the single pre-existing invalid changepack key may be corrected so hooks can pass. Hooks have not been bypassed; package/plugin behavior and the baseline changepack remain untouched pending that answer |

The metadata failure, local coverage gap, and landing nondeterminism are separately reported to the coordinator. They are not silently folded into an experiment-only core/plugin change. No new application build errors are introduced by W72. The final branch/commit status must be read from the delivery message, not inferred from a successful build exit code or this checkpoint section.
