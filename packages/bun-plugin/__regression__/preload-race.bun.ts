import { expect, it } from 'bun:test'

// NOTE: this file is intentionally named `*.bun.ts`, NOT `*.test.ts`, so the
// root test suite (root = "packages", source preload + 100% coverage gate) does
// NOT auto-discover it. It must run via its own bunfig that preloads the BUILT
// plugin (dist/index.mjs) — invoke it through `bun run --filter
// @devup-ui/bun-plugin test:regression`.
//
// This static, top-level import is the crux of the regression: bun resolves it
// while loading this file during the preload/collection phase. The fixture
// calls a compile-only @devup-ui/react API at its own top level, so it only
// loads without throwing if the preloaded plugin already installed its onLoad
// hook. Before the fix (un-awaited Bun.plugin registration) this import raced
// the async setup() and threw "Cannot run on the runtime".
import { fade, transitive } from './fixtures/entry'

it('rewrites top-level compile-only API before the preloaded plugin races its async setup', () => {
  // keyframes() is rewritten to a static class string by the plugin transform.
  expect(typeof fade).toBe('string')
  expect(fade.length).toBeGreaterThan(0)
  // The transitive chain still resolves, confirming the larger graph loaded.
  expect(transitive).toBe('edcba')
})
