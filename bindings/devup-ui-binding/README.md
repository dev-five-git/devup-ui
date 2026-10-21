# Native build-host adapter (WIP)

`adapter.cjs` privately exports `createBinding(raw)`; its returned object explicitly exposes the existing 29 WASM names. Raw addon helpers, positional requests and `interop.cjs` are not package ABI. No public loader or npm manifest is provided yet.

The synchronous Unknown-backed decoder calls the existing engine Serde implementations, without JSON preconversion. JSON.stringify is used only after invalid-type selection for the generated-WASM debug diagnostic. Resolver handles are per environment; replacement, both resets and teardown release them. Native state still does not support concurrent independent builds.

The decoder is private and target-bounded: unused Serde hints use trait forwarding, not an additional public compatibility API. New engine Deserialize shapes require fresh boundary and real-host coverage evidence.

Malformed prefix policy `msg_f4283cd7dea1` is unresolved. Valid strings/nullish values are implemented; nonstrings currently encounter native argument rejection, not emulated WASM traps. This is not all-input parity or acceptance. No builds/tests/coverage have run for this implementation; main must perform same-revision Node/Bun differentials and real Node standard-tarpaulin attribution, including private error paths and finalization.
