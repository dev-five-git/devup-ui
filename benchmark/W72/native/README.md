# W72 synchronous native bridge (private experiment)

Use this directory's `index.cjs` as the replacement module, **not** the raw
`.node` addon. The entrypoint adapts JS Maps/Sets and absent option values to
the addon's JSON boundary and preserves the WASM resolver's exception handling.
It is build-tool glue, not a browser styling runtime. There are no npm dependencies.

This crate is its own workspace. All three core path dependencies resolve through
`../../../libs/`: native → W72 → benchmark → repository root → libs.
It does not depend on the WASM crate or change any published package.

## Build

Run serially from the repository root under the coordinator's 6 GiB watchdog:

```powershell
$env:CARGO_BUILD_JOBS='2'; cargo build --manifest-path benchmark/W72/native/Cargo.toml --release
```

On Windows, stage `benchmark/W72/native/target/release/devup_ui_w72_native.dll`
as `benchmark/W72/native/devup_ui_w72_native.node`. On Linux use
`libdevup_ui_w72_native.so`; on macOS use `libdevup_ui_w72_native.dylib`.
If `CARGO_TARGET_DIR` is set, obtain the library from that target directory instead.
The optional size-comparison build replaces `--release` with `--profile optz`;
the artifact is then under `target/optz`.

Require `benchmark/W72/native/index.cjs` in Node/Bun, or alias `@devup-ui/wasm`
to that entrypoint in the coordinator's harness. Native addon filenames alone
are not automatically recognized as JS modules until staged with `.node`.

## API and semantics

The entrypoint exposes all 27 top-level WASM functions and `Output`:

- Extraction: `codeExtract`, `codeExtractWithoutSourceMap`, `hasDevupUI`.
- Sheet/theme: `getCss`, `registerTheme`, `registerShorthands`,
  `getDefaultTheme`, `getThemeInterface`.
- State: `importSheet`, `exportSheet`, `importClassMap`, `exportClassMap`,
  `importFileMap`, `exportFileMap`, `importCanonicalMap`, `exportCanonicalMap`,
  `importFileRoutes`, `seedFileMap`, `resetBuildState`.
- Configuration: `setDebug`, `isDebug`, `setPrefix`, `getPrefix`,
  `setAtomHoist`, `setModuleResolver`.
- Reservation: `reserveClassSlots`, `getUnreservedClassSlotCount`.

`Output` getters are `code`, `map`, `cssFile`, `css`, `updatedBaseStyle`,
`dependencies`; absent strings return `undefined`, not `null`. `free()` eagerly
releases strings and getters thereafter throw an Error. Repeated `free()` is
idempotent; post-free error text and WASM pointer internals are not reproduced.
`Symbol.dispose` aliases `free` when supported. Like WASM, no public constructor
is provided. Prefix/default-theme absence also returns `undefined`.

Both extraction modes, output collection/removal order, canonical/raw filename
handling, atom-mode scope, snapshot sorting, reservation seeding, scratch guards
and reset ordering follow the binding source. Reservation row arity is validated
for the entire packet before any mutation; per-file extraction failures are
suppressed. Resolver errors/malformed results are misses. Callbacks receive
`this = null`, run synchronously on the invoking JS thread, and are retained by
a thread-local `Rc<FunctionRef>`; no custom unsafe code or Send/Sync impl exists.
Reset drops the installed resolver; an in-flight extraction retains its snapshot.

## Boundaries and unverified caveats

- The W72 report contains the actual Windows build, Node/Bun API, output and
  performance evidence. It is not production migration signoff; raw-cache and
  landing production identity limitations remain explicit.
- One JS environment/build at a time per process. Do not share state through Node
  workers or call extraction concurrently. The Rust core retains its existing
  globals; the callback reference never migrates to another thread.
- Ordinary objects, nested string-key Maps, arrays, Sets and nullable aliases are
  adapted without JSON.stringify/toJSON. Present undefined object/map/array
  values become null for serde Option handling; missing fields stay missing. Rust parses
  those values into the same core types. Prototype/symbol keys are not copied.
- Exotic objects, custom iterables, non-finite numbers, BigInt and function-valued
  unknown fields are not guaranteed to match serde-wasm-bindgen. JSON/napi parse
  error wording differs even though malformed input still throws an Error.
  Cyclic inputs throw. Imported state is the parsed export string, not the string.
- Native std HashMap randomization is unchanged. Exact emitted code/CSS and
  exported-state identity versus WASM must be measured by the harness, especially
  theme maps. No sorting algorithm was substituted to mask this difference.
- `panic = abort` terminates the host on a Rust panic; located extractor failures
  remain ordinary JS Errors. No stack-size overrides or async work are introduced.
- Release is opt-level 3/LTO/one codegen unit; `optz` changes only opt-level to z.
  The edition floor is 1.85; current core/dependencies require a newer compiler,
  so use current stable Rust. The recorded measurements used stable 1.99.1;
  that observation is not a toolchain pin.

## Documentation checked

Latest docs.rs endpoints checked on 2026-10-19 reported napi 3.14.2,
napi-derive 3.6.12 and napi-build 2.6.0. Cargo constraints are compatible lower
bounds, not exact pins.

- <https://docs.rs/napi/latest/napi/bindgen_prelude/struct.FunctionRef.html>
- <https://docs.rs/napi/latest/napi/bindgen_prelude/type.Undefined.html>
- <https://docs.rs/napi-derive/latest/napi_derive/>
- <https://docs.rs/napi-build/latest/napi_build/>
- <https://raw.githubusercontent.com/napi-rs/napi-rs/main/crates/napi/src/bindgen_runtime/js_values/function.rs>
- <https://raw.githubusercontent.com/napi-rs/napi-rs/main/crates/napi/src/bindgen_runtime/js_values/serde.rs>

The source checks confirm `FnArgs`, `FunctionRef::borrow_back`, synchronous
`Function::call`, and why raw napi JSON conversion alone cannot accept JS Maps
or undefined values. Context7 was not retried because its monthly quota is
already exhausted per the task brief.
