import { mkdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { nativeFixture } from '../../../../test-harness/native-fixture'
import { nativePublicationFixture } from '../../../../test-harness/native-publication-fixture'
import {
  acceptPublication,
  beginPublication,
  recordRoot,
  recordVite,
} from '../../../../test-harness/publication'
import {
  sourceFiles,
  viteRequired,
} from '../../../../test-harness/source-inventory'

it('snapshots source inputs recursively while keeping generated artifacts outside identity', () => {
  const fixture = nativeFixture({})
  try {
    for (const path of [
      'src/nested',
      'src/pkg',
      'src/dist',
      'src/node_modules',
      'src/.hidden',
    ]) {
      mkdirSync(join(fixture.root, path), { recursive: true })
      writeFileSync(join(fixture.root, path, 'file.ts'), 'export {}')
    }
    writeFileSync(join(fixture.root, 'src/readme.md'), 'not executable')
    writeFileSync(join(fixture.root, 'src/config.json'), '{}')
    expect(sourceFiles(fixture.root, 'src')).toEqual([
      'src/config.json',
      'src/nested/file.ts',
    ])
    const required = viteRequired(resolve(import.meta.dir, '../../../..'))
    expect(required).toContain('packages/vite-plugin/src/resolution-watch.ts')
    expect(required).toContain(
      'packages/vite-plugin/src/production-root-closure.ts',
    )
    expect(required).toContain(
      'packages/vite-plugin/src/__tests__/resolution-watch-subscription.ts',
    )
  } finally {
    fixture.dispose()
  }
})

it('keeps default source requirements active rather than accepting a passing but missing inventory', async () => {
  const fixture = nativePublicationFixture()
  try {
    mkdirSync(join(fixture.root, 'packages/vite-plugin/src'), {
      recursive: true,
    })
    beginPublication(fixture.root)
    expect((await fixture.lane('root')).code).toBe(0)
    expect((await fixture.lane('vite')).code).toBe(0)
    expect(() => recordRoot(fixture.root)).toThrow('missing source')
    expect(() => recordVite(fixture.root)).toThrow('missing source')
    expect(() => acceptPublication(fixture.root)).toThrow('missing source')
  } finally {
    fixture.dispose()
  }
}, 120000)
