import { join } from 'node:path'

import { it } from 'bun:test'

import { ownedNativeFixture } from './serve-coherence-owned-native-fixture'

it('retains only the newer real flight when independent config load finishes before old write', () => {
  ownedNativeFixture(
    join(import.meta.dir, 'cache-invalidation-config-capture-native.cjs'),
    [],
    () => {},
  )
}, 180000)
