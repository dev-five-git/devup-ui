import { join } from 'node:path'

import { it } from 'bun:test'

import { retainedNativeFixture } from './provenance-owned-native-fixture'

it('reextracts outside inherited theme edit delete recreate on actual native events', () => {
  retainedNativeFixture(
    join(import.meta.dir, 'cache-invalidation-outside-theme-native.cjs'),
    [],
  )
}, 180000)
