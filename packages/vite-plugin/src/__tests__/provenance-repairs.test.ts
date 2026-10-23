import { join } from 'node:path'

import { it } from 'bun:test'

import { retainedNativeFixture } from './provenance-owned-native-fixture'
import { ownedNativeFixture } from './serve-coherence-owned-native-fixture'

for (const mode of [
  'fresh',
  'replacement',
  'pending-replacement',
  'physical',
  'physical-query',
]) {
  it(`preserves real extraction lifetime when ${mode} occurs`, () => {
    // Given / When: execute the actual facade in an owned Node process.
    ownedNativeFixture(
      join(import.meta.dir, 'provenance-lifetime.cjs'),
      [mode],
      () => {
        // Then: child assertions pass before confirmed tree_exit/active[] cleanup.
      },
    )
  }, 180000)
}

for (const mode of ['watchChange', 'hotUpdate', 'handleHotUpdate']) {
  it(`distinguishes unrelated JSON from actual dependencies through ${mode}`, () => {
    ownedNativeFixture(
      join(import.meta.dir, 'provenance-admission.cjs'),
      [mode],
      () => {},
    )
  }, 180000)
}

for (const mode of ['exports', 'outside-native']) {
  it(`preserves provenance when real ${mode} changes`, () => {
    if (mode === 'outside-native') {
      retainedNativeFixture(join(import.meta.dir, 'provenance-boundary.cjs'), [
        mode,
      ])
      return
    }
    ownedNativeFixture(
      join(import.meta.dir, 'provenance-boundary.cjs'),
      [mode],
      () => {},
    )
  }, 180000)
}
