import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  ConfigLoadError,
  loadDevupConfig,
  loadDevupConfigSync,
} from '../load-config'

const parent = join(
  tmpdir(),
  'opencode',
  'workers',
  'w20-plugins-core',
  'shared',
)
let root: string
beforeEach(() => {
  mkdirSync(parent, { recursive: true })
  root = realpathSync(mkdtempSync(join(parent, 'config-')))
})
afterEach(() => rmSync(root, { recursive: true, force: true }))

describe('located bounded config errors', () => {
  it.each(['{', 'null', '[]', '{"extends":"base.json"}', '{"extends":[1]}'])(
    'rejects invalid config %s in both loaders',
    async (content) => {
      const file = join(root, 'devup.json')
      writeFileSync(file, content)
      expect(() => loadDevupConfigSync(file)).toThrow(ConfigLoadError)
      await expect(loadDevupConfig(file)).rejects.toThrow(file)
    },
  )
  it('reports a missing parent rather than silently clearing its theme', async () => {
    const file = join(root, 'devup.json')
    const missing = join(root, 'missing.json')
    writeFileSync(file, JSON.stringify({ extends: ['./missing.json'] }))
    expect(() => loadDevupConfigSync(file)).toThrow(missing)
    await expect(loadDevupConfig(file)).rejects.toThrow(missing)
  })
  it('reports directory read errors with the file and original cause', async () => {
    try {
      loadDevupConfigSync(root)
      throw new Error('Expected a read failure')
    } catch (error) {
      expect(error).toBeInstanceOf(ConfigLoadError)
      if (!(error instanceof ConfigLoadError)) throw error
      expect(error.file).toBe(root)
      expect(error.cause).toBeInstanceOf(Error)
    }
    await expect(loadDevupConfig(root)).rejects.toThrow(root)
  })
  it('detects canonical inheritance cycles in both loaders', async () => {
    const file = join(root, 'a.json')
    writeFileSync(file, JSON.stringify({ extends: ['./b.json'] }))
    writeFileSync(
      join(root, 'b.json'),
      JSON.stringify({ extends: ['./nested/../a.json'] }),
    )
    mkdirSync(join(root, 'nested'))
    expect(() => loadDevupConfigSync(file)).toThrow('a.json ->')
    await expect(loadDevupConfig(file)).rejects.toThrow('a.json ->')
  })
  it('allows repeated parents across separate branches of a diamond', async () => {
    writeFileSync(
      join(root, 'base.json'),
      JSON.stringify({ theme: { colors: { default: { text: 'red' } } } }),
    )
    writeFileSync(
      join(root, 'left.json'),
      JSON.stringify({ extends: ['./base.json'] }),
    )
    writeFileSync(
      join(root, 'right.json'),
      JSON.stringify({ extends: ['./base.json'] }),
    )
    const file = join(root, 'devup.json')
    writeFileSync(
      file,
      JSON.stringify({ extends: ['./left.json', './right.json'] }),
    )
    const expected = { theme: { colors: { default: { text: 'red' } } } }
    expect(loadDevupConfigSync(file)).toEqual(expected)
    expect(await loadDevupConfig(file)).toEqual(expected)
  })
  it('returns an empty config for a missing top file in both loaders', async () => {
    const file = join(root, 'missing.json')
    expect(loadDevupConfigSync(file)).toEqual({})
    expect(await loadDevupConfig(file)).toEqual({})
  })
})
