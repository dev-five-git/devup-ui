import { join } from 'node:path'

import { it } from 'bun:test'

import { ownedNativeFixture } from './serve-coherence-owned-native-fixture'

for (const hook of ['watchChange', 'hotUpdate', 'handleHotUpdate']) {
  for (const mode of ['root', 'inherited', 'tailwind', 'tailwind-import']) {
    it(`reextracts real cached ${mode} semantics when ${hook} completes`, () => {
      ownedNativeFixture(
        join(import.meta.dir, 'cache-invalidation-metadata-native.cjs'),
        [mode, hook],
        () => {},
      )
    }, 180000)
  }
}
