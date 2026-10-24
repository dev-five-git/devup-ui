import { join } from 'node:path'

import { it } from 'bun:test'

import { ownedNativeFixture } from './serve-coherence-owned-native-fixture'

it('loads actual CSS when a full transform query contains path separators', () => {
  ownedNativeFixture(
    join(import.meta.dir, 'cache-invalidation-query-native.cjs'),
    [],
    () => {},
  )
}, 180000)
