import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import {
  ConfigLoadError,
  loadDevupConfig,
  loadDevupConfigSync,
} from '../load-config'

for (const load of [loadDevupConfigSync, loadDevupConfig]) {
  it(`reports actual nested inputs when using ${load.name}`, async () => {
    // Given
    const root = mkdtempSync(join(tmpdir(), 'serve-coherence-config-'))
    const file = (name: string) => join(root, name)
    writeFileSync(file('root.json'), '{"extends":["./base.json"]}')
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
        ['root.json', 'base.json', 'nested.json'].map(file),
      )
      expect(config.theme?.colors).toEqual({
        default: { primary: 'red', background: 'white' },
      })
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })

  it(`reports a missing extended input before rejecting with ${load.name}`, async () => {
    // Given
    const root = mkdtempSync(join(tmpdir(), 'serve-coherence-missing-config-'))
    const file = join(root, 'root.json')
    const missing = join(root, 'missing.json')
    writeFileSync(file, '{"extends":["./missing.json"]}')
    const inputs = new Set<string>()
    try {
      // When
      const result = Promise.resolve().then(() => load(file, inputs))
      // Then
      await expect(result).rejects.toBeInstanceOf(ConfigLoadError)
      await expect(result).rejects.toMatchObject({ file: missing })
      expect([...inputs]).toEqual([file, missing])
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })
}
