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
No install hooks are used.

## Native CI and release

`publish.yml` adds an independent read-only lane: eight fresh optimized builds,
one full-nine pack collector, then eight consumers of the **same** tarballs.
It does not block existing validation/benchmark jobs; main release additionally
requires all native consumers. Linux ARM consumers are QEMU emulation; macOS
x64 uses Rosetta with both x64 Node and Bun. Windows ARM builds and consumes
on the newest exposed real ARM runner (currently `windows-11-vs2026-arm`).
Linux uses current GNU/Alpine Node images and digest-verified latest official
Bun native/musl assets. Each container executes its installed TypeScript binary
before declaration checks; an incompatible binary fails, never skips the gate.
Receipts retain effective jobs2/musl flags, raw source hashes, actual images,
tools, build logs and target inspection. No compiled-target cache is reused.

Repository scripts (arguments are forwarded unchanged):

```sh
bun run build:native:artifact <triple> <fresh-artifact-dir> <run-id/1> [--zigbuild]
bun run pack:native <eight-artifact-dirs> <fresh-bundle-dir> <run-id/1>
bun run test:native:package
bun run test:native:consumer <bundle-dir> <suffix> <report> <tsc-executable> <npm-cli.js>
```

Package tests require `NATIVE_PROOF_ARTIFACTS` (a directory containing exactly
one real host artifact directory) and `NATIVE_PROOF_BUNDLE` (its verified bundle,
full-nine in CI). Explicit dirty/host-only flags are local proof modes, not
release acceptance. Collector first needs `pkg/index.cjs` generated with the
public producer. All output directories must live outside the checkout.

Per-manifest changepacks overrides invoke `publish.mjs` from the selected native
package directory; other packages retain their existing commands. Set
`NATIVE_RELEASE_BUNDLE` to the downloaded immutable full bundle. `dry-run` is
local source/payload/manifest validation only: no registry query, lifecycle or
`npm publish --dry-run`. Real mode requires this repository's main push,
`publish.yml` workflow identity and OIDC environment. It publishes only the
selected absolute tarball with scripts disabled; root requires all eight exact
registry versions **and** matching integrities first. Changepacks owns selection,
dependency ordering and release receipts; the dispatcher never adds siblings.

Source validation happens inside each invocation, after the action's possible
sourceSha checkout. Every recorded input byte/commit/tree must match; only its
untracked `changepacks`/`changepacks.exe` tool is tolerated. Locks are neither
hidden, restored nor regenerated during publication. Before installation, build
or changepacks, every release (including JS-only) checks workspace membership,
versions and dependency specifications against `bun.lock` without writing source
or lock files. A mismatch fails: run `bun install` on the Update Versions branch
and commit `bun.lock` before release.

Artifacts retain seven days. A release-only retry reuses `native-packs-<run-id>`
and original producer identity `<run-id>/1`, regardless of release run_attempt.
Do not rerun build/collector in that run: they reject attempts after1 and cannot
overwrite original payloads. A missing/expired or incomplete bundle needs a new
fresh workflow run; older source intents fail closed rather than using current
images. No retry may silently replace a partially published release's payloads.

Before first automated0.1.1, the owner must register all nine exact npm names
and configure direct npm publish permission/Trusted Publishers for this repo's
`publish.yml`; workers never publish or consume that version. Foreign builds,
full-nine runtime execution, OIDC and row timings are CI-only until actually run.
No plugin/default-engine change is authorized by this integration.

The synchronous Unknown-backed decoder calls the existing engine Serde implementations, without JSON preconversion. JSON.stringify is used only after invalid-type selection for the generated-WASM debug diagnostic. Resolver handles are per environment; replacement, both resets and teardown release them. Native state still does not support concurrent independent builds.

The decoder is private and target-bounded: unused Serde hints use trait forwarding, not an additional public compatibility API. New engine Deserialize shapes require fresh boundary and real-host coverage evidence.

Malformed-enum error paths follow shipped serde-wasm-bindgen 0.6.5: Object.entries result length is read and coerced once for the check, then read and coerced again for the invalid-length diagnostic when the first length is not one, before any index read. The second value supplies the diagnostic; errors thrown by its getter or coercion propagate unchanged. Valid enums retain one length read.

The shared native/WASM boundary validates ordinary calls at 45 argument positions in 18 functions: malformed inputs throw a genuine `TypeError` naming the function, parameter and required type before state mutation. Preflight defaults the entire alias map on schema failure (after diagnostics) or a caught iterator-import failure whose `new Error(String(error))` conversion succeeds. Raw access failures and conversion faults escape unchanged before resolver selection; conversion runs once with the pending failure cleared, so reentry is isolated and resolver changes take effect before the outer core starts. Extraction and reservation remain strict. For example, `setPrefix(false)` throws `setPrefix(): argument prefix must be a string, null or undefined`; string/nullish prefixes remain supported. Extraction wrappers retain the measured metadata, receiver, extra-argument and apply/construct policy; `seedFileMap` validates and copies the caller's array before engine entry without rereading its indices. Same-revision Node/Bun identity, 267-case invalid-input, 73-exception preservation and seed-safety gates cover these contracts, not optimized shipping-native performance or real Node-host Linux Rust/authored-JS 100% coverage, which remain public-delivery gates.
