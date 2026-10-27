import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { it } from 'bun:test'

import { ownedNativeFixture } from './serve-coherence-owned-native-fixture'

it('recovers a real production watcher after a newly attempted inherited config is created', () => {
  ownedNativeFixture(
    resolve(import.meta.dir, 'production-config-refresh-native.cjs'),
    [],
    (root) => {
      assert(existsSync(join(root, 'project/src/A.tsx')))
    },
  )
}, 180000)

it.each(['missing', 'invalid'])(
  'recovers a real production watcher when its initial outside inherited config is %s',
  (mode) => {
    ownedNativeFixture(
      resolve(import.meta.dir, 'production-config-initial-native.cjs'),
      [mode],
      (root) => {
        console.info(
          readFileSync(join(root, 'initial-watch-evidence.json'), 'utf8'),
        )
      },
    )
  },
  180000,
)
