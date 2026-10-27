# Native build-host adapter (WIP)

`adapter.cjs` privately exports `createBinding(raw)`; its returned object explicitly exposes the existing 29 WASM names. Raw addon helpers, positional requests and `interop.cjs` are not package ABI. No public loader or npm manifest is provided yet.

The synchronous Unknown-backed decoder calls the existing engine Serde implementations, without JSON preconversion. JSON.stringify is used only after invalid-type selection for the generated-WASM debug diagnostic. Resolver handles are per environment; replacement, both resets and teardown release them. Native state still does not support concurrent independent builds.

The decoder is private and target-bounded: unused Serde hints use trait forwarding, not an additional public compatibility API. New engine Deserialize shapes require fresh boundary and real-host coverage evidence.

The shared native/WASM boundary validates ordinary calls at 45 argument positions in 18 functions: malformed inputs throw a genuine `TypeError` naming the function, parameter and required type before state mutation, while preflight aliases retain their forgiving policy. For example, `setPrefix(false)` throws `setPrefix(): argument prefix must be a string, null or undefined`; string/nullish prefixes remain supported. Extraction wrappers retain the measured metadata, receiver, extra-argument and apply/construct policy; `seedFileMap` validates and copies the caller's array before engine entry without rereading its indices. Same-revision Node/Bun identity, 267-case invalid-input, 73-exception preservation and seed-safety gates cover these contracts, not optimized shipping-native performance or real Node-host Linux Rust/authored-JS 100% coverage, which remain public-delivery gates.
