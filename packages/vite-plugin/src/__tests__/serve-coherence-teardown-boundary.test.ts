import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { it } from 'bun:test'

import { ownedNativeFixture } from './serve-coherence-owned-native-fixture'

it.skipIf(process.platform !== 'win32')(
  'removes a Windows held-directory fixture only after its owned tree exits',
  () => {
    // Given / When: a real descendant denies the old live-owner deletion boundary.
    ownedNativeFixture(
      resolve(import.meta.dir, 'serve-coherence-teardown-boundary.cjs'),
      ['owner'],
      (root) => {
        // Then: receipt inspection and parent deletion happen only after owned exit.
        const receipt: unknown = JSON.parse(
          readFileSync(join(root, 'boundary.json'), 'utf8'),
        )
        assert(typeof receipt === 'object' && receipt !== null)
        assert('denied' in receipt && typeof receipt.denied === 'string')
        assert(['EPERM', 'EBUSY', 'EACCES'].includes(receipt.denied))
        assert('root' in receipt)
        assert.equal(receipt.root, root)
      },
    )
  },
  180000,
)
