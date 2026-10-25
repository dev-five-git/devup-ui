import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { nativePublicationFixture } from '../../../../test-harness/native-publication-fixture'
import {
  acceptPublication,
  beginPublication,
  continuePublication,
  digest,
  recordRoot,
  recordVite,
} from '../../../../test-harness/publication'

it('consumes fresh ROOT authority once before the required Vite measurement', async () => {
  // Given a real successful ROOT lane and its pending continuation.
  const fixture = nativePublicationFixture()
  try {
    beginPublication(fixture.root)
    expect((await fixture.lane('root')).code).toBe(0)
    recordRoot(fixture.root, ['source.ts'])
    const report = join(fixture.root, 'coverage/root/lcov.info')
    const before = digest(readFileSync(report))
    // When the continuation consumes that authority before starting Vite.
    continuePublication(fixture.root, ['source.ts'])
    // Then it cannot replay or alter the ROOT bytes and needs a fresh Vite result.
    expect(() => continuePublication(fixture.root, ['source.ts'])).toThrow()
    expect(digest(readFileSync(report))).toBe(before)
    expect(() =>
      acceptPublication(fixture.root, {
        root: ['source.ts'],
        vite: ['source.ts'],
      }),
    ).toThrow()
    expect((await fixture.lane('vite')).code).toBe(0)
    recordVite(fixture.root, ['source.ts'])
    acceptPublication(fixture.root, {
      root: ['source.ts'],
      vite: ['source.ts'],
    })
    expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(true)
  } finally {
    fixture.dispose()
  }
}, 120000)

it.each(['report', 'success-marker'] as const)(
  'rejects a changed ROOT %s before continuing a real successful lane',
  async (changed) => {
    // Given genuine complete native ROOT observations and their success markers.
    const fixture = nativePublicationFixture()
    try {
      beginPublication(fixture.root)
      expect((await fixture.lane('root')).code).toBe(0)
      recordRoot(fixture.root, ['source.ts'])
      const file = join(
        fixture.root,
        changed === 'report'
          ? 'coverage/root/lcov.info'
          : 'coverage/root.sha256',
      )
      writeFileSync(file, readFileSync(file, 'utf8') + '\n')
      // When continuation sees different report bytes or success custody.
      const action = () => continuePublication(fixture.root, ['source.ts'])
      // Then mismatched native authority rejects without an accepted publication.
      expect(action).toThrow(
        'ROOT continuation no longer matches native success',
      )
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        false,
      )
    } finally {
      fixture.dispose()
    }
  },
  120000,
)
