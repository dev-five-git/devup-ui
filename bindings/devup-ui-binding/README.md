# @devup-ui/binding

Synchronous native build-time compiler for Node.js and Bun. Both
`require('@devup-ui/binding')` and named ESM imports expose exactly the existing
29 compiler names, with identical function/class identities and no default init.
Authored CommonJS/ESM declarations are included; no WASM dependency is required.

The private loader selects one optional platform sibling: Windows MSVC, macOS,
or Linux GNU/musl, on x64/arm64. Install with optional dependencies enabled.
Missing packages name the required sibling; genuine image/load errors propagate.
There is no browser, WASI, download or silent WASM fallback. Only `.` is exported;
raw addon helpers, positional requests, adapter and interop are not package ABI.

From the repository, build an explicit target with
`node bindings/devup-ui-binding/build.mjs --target x86_64-pc-windows-msvc`.
The driver uses locked Cargo, optimized `native` profile and two jobs; removes
the selected previous source/copy, then copies only that target's fresh image
to its sibling. It also generates package-relative `pkg/index.cjs`.
Linux cross builds may append `--zigbuild` (current cargo-zigbuild/Zig required;
GNU compatibility floor glibc 2.28); musl uses dynamic linking, not crt-static.
The canonical eight targets are in `e2e/binding/package/matrix.mjs`.
The existing `node script.cjs <absolute-addon> <absolute-destination>` private
producer remains unchanged in behavior; `--public <absolute-destination>` is
the separate relocatable mode. Generated files and images are not source.

All nine manifests seed 0.1.0 with same-release workspace optional edges;
the release pipeline must build all targets and materialize exact versions.
This foundation does not prove foreign images, release packing or publication.
No install hooks are used.

The synchronous Unknown-backed decoder calls the existing engine Serde implementations, without JSON preconversion. JSON.stringify is used only after invalid-type selection for the generated-WASM debug diagnostic. Resolver handles are per environment; replacement, both resets and teardown release them. Native state still does not support concurrent independent builds.

The decoder is private and target-bounded: unused Serde hints use trait forwarding, not an additional public compatibility API. New engine Deserialize shapes require fresh boundary and real-host coverage evidence.

Malformed-enum error paths follow shipped serde-wasm-bindgen 0.6.5: Object.entries result length is read and coerced once for the check, then read and coerced again for the invalid-length diagnostic when the first length is not one, before any index read. The second value supplies the diagnostic; errors thrown by its getter or coercion propagate unchanged. Valid enums retain one length read.

The shared native/WASM boundary validates ordinary calls at 45 argument positions in 18 functions: malformed inputs throw a genuine `TypeError` naming the function, parameter and required type before state mutation. Preflight defaults the entire alias map on schema failure (after diagnostics) or a caught iterator-import failure whose `new Error(String(error))` conversion succeeds. Raw access failures and conversion faults escape unchanged before resolver selection; conversion runs once with the pending failure cleared, so reentry is isolated and resolver changes take effect before the outer core starts. Extraction and reservation remain strict. For example, `setPrefix(false)` throws `setPrefix(): argument prefix must be a string, null or undefined`; string/nullish prefixes remain supported. Extraction wrappers retain the measured metadata, receiver, extra-argument and apply/construct policy; `seedFileMap` validates and copies the caller's array before engine entry without rereading its indices. Same-revision Node/Bun identity, 267-case invalid-input, 73-exception preservation and seed-safety gates cover these contracts, not optimized shipping-native performance or real Node-host Linux Rust/authored-JS 100% coverage, which remain public-delivery gates.
