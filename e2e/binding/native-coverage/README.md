# Production native coverage

`bun run test` builds the actual production addon with LLVM instrumentation,
copies it outside the Cargo target directory, and generates its entry with the
normal binding producer. The root tarpaulin invocation enables the test-only
`devup-ui-binding/native-coverage` feature and loads that same addon through
`--objects`. Its ordinary XML and stdout reports include the genuine Node
profiles alongside the Rust tests. CI still requires 100 percent coverage.

The production crate has no code conditional on this feature. Plain
`cargo test --workspace` and `cargo test --workspace --all-targets` require no
preparation and do not run the Node integration target. Explicitly enabling
`native-coverage` without the root preparer's environment fails with an
actionable error; it never skips.

The integration target waits for real Node children to exit naturally. They run
the existing public extraction/entry and argument-validation scenarios, followed
by the narrow private protocol suite. Successful decoding uses the authored
interop. Synthetic envelopes are limited to targeted private protocol failures.
The raw resolver failure group is separately named for the private acceptance
experiment, but the normal root entry always executes both groups.
The prepared host path is test-only: the root preparer always replaces it with
the positive scenarios script, never an inherited alternate host or control mode.

The root preparer prints its unique temporary evidence directory. It retains the
production `.node`, generated entry, build profiles, child records and `run.json`
with commands, process IDs, statuses, durations and the addon hash. Runtime addon
profiles inherit tarpaulin's `%m`/`%p` path under its own target profile directory.
No report is replaced or manually merged. Full-source JSON is produced only by
the private evidence experiment, outside the linted repository root. Ordinary Node and the installed
latest stable Rust/tarpaulin are used on Windows and Linux.

This instrumented development build is coverage evidence, not a shipping-native
performance measurement. Real-engine parity and optimized hosting measurements
remain separate gates. Native line totals can differ across platforms and CI's
wide source formatting; inspect the actual report rather than equating a local
percentage with the Linux CI result.

The retained coverage spike has been retired (exact workflow and eleven fixture
files); production coverage above is unchanged. Standard Linux CI100% remains
the final authority. The inherited Windows98.09% mapping is local evidence only,
not a replacement for that gate. The parallel `native-build`/`native-packs`/
`native-consumer` shipping lane in `publish.yml` tests optimized public packages,
not instrumented coverage or native performance. Its results cannot waive the
production Linux coverage gate.
