# W74a native coverage SPIKE

Standalone scratch workspace, not production/root-package coverage. Rust sources and
lock copy the final lint-clean private probe; the CJS scenario is unchanged semantically.
With current Node, Bun, stable Rust and cargo-tarpaulin already available, run:

```sh
node e2e/binding/coverage-spike/run.mjs /absolute/private/evidence
```

A unique evidence child holds the exact instrumented `.node`, raw JSON/XML/logs,
commands, profiles and hashes. Node-negative must pass the host and fail the unchanged
100% threshold; Node/Bun-positive must cover all 17 addon + 2 core lines. Only sentinel
55–57 miss in negative. Mapping discrepancies fail with evidence, never a weaker gate.
`build.rs`/`native_host.rs` 0/0 are visibly unmeasured scaffolding, not measured coverage.
Hit magnitudes are only zero/nonzero. Linux acceptance and actual-package coverage
remain main-agent gates; replace/remove the SPIKE workflow before the PR leaves Draft.
