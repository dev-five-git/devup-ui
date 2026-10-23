import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { ownedNativeFixture } from './serve-coherence-owned-native-fixture'

for (const mode of [
  'basic',
  'input',
  'query',
  'sheet',
  'build',
  'membership',
]) {
  it(`qualifies real cache ${mode} when the owned session transforms`, () => {
    // Given / When: the owned Node child executes two real facade transforms.
    ownedNativeFixture(
      join(import.meta.dir, 'serve-coherence-cache-native.cjs'),
      [mode],
      (root) => {
        // Then: the fixture survives until the original owner confirms tree exit.
        expect(readFileSync(join(root, 'src/A.tsx'), 'utf8')).toContain('Box')
      },
    )
  }, 180000)
}

for (const mode of [
  'root',
  'inherited',
  'tailwind',
  'tailwind-import',
  'alias',
  'exports',
]) {
  it(`invalidates real cached output when ${mode} metadata changes`, () => {
    ownedNativeFixture(
      join(import.meta.dir, 'serve-coherence-cache-metadata.cjs'),
      [mode],
      () => {},
    )
  }, 180000)
}

for (const mode of ['flight', 'reconfigure', 'refresh', 'close']) {
  it(`protects real pending output when ${mode} occurs`, () => {
    ownedNativeFixture(
      join(import.meta.dir, 'serve-coherence-cache-lifetime.cjs'),
      [mode],
      () => {},
    )
  }, 180000)
}
it('bypasses reuse when a second actual configuration overlaps', () => {
  ownedNativeFixture(
    join(import.meta.dir, 'serve-coherence-cache-ownership.cjs'),
    [],
    () => {},
  )
}, 180000)
