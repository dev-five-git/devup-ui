import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { it } from 'bun:test'

import { ownedNativeFixture } from './serve-coherence-owned-native-fixture'

for (const [probe, ...args] of [
  ['native', 'theme'],
  ['native', 'alias'],
  ['ownership', 'esm', 'esm'],
  ['ownership', 'esm', 'cjs'],
  ['ownership', 'cjs', 'cjs'],
  ['ownership', 'cjs', 'esm'],
]) {
  it(`preserves real prerequisite behavior when ${probe} ${args.join(' ')}`, () => {
    // Given / When: real built plugins, filesystem events and WASM Output.
    ownedNativeFixture(
      resolve(import.meta.dir, `serve-coherence-prereq-${probe}.cjs`),
      args,
      (root) => {
        // Then: only the parent may remove the populated fixture after tree exit.
        assert(
          existsSync(join(root, 'src/A.tsx')),
          'parent fixture must survive the owned child',
        )
      },
    )
  }, 180000)
}
