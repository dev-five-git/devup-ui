import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { loadDevupConfig, loadDevupConfigSync } from '../load-config'

for (const load of [loadDevupConfigSync, loadDevupConfig]) {
  it(`reports actual nested and missing inputs when using ${load.name}`, async () => {
    // Given
    const root = mkdtempSync(join(tmpdir(), 'serve-coherence-config-'))
    const file = (name: string) => join(root, name)
    writeFileSync(
      file('root.json'),
      '{"extends":["./base.json","./missing.json"]}',
    )
    writeFileSync(
      file('base.json'),
      '{"extends":["./nested.json"],"theme":{"colors":{"default":{"primary":"red"}}}}',
    )
    writeFileSync(
      file('nested.json'),
      '{"theme":{"colors":{"default":{"background":"white"}}}}',
    )
    const inputs = new Set<string>()
    try {
      // When
      const config = await load(file('root.json'), inputs)
      // Then
      expect([...inputs]).toEqual(
        ['root.json', 'base.json', 'nested.json', 'missing.json'].map(file),
      )
      expect(config.theme?.colors).toEqual({
        default: { primary: 'red', background: 'white' },
      })
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })
}
