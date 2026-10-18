import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { afterEach, describe, expect, it } from 'bun:test'

import { resolveWasmBindgenVersion } from '../../../../wasm-bindgen-version.js'

const resolver = fileURLToPath(
  new URL('../../../../resolve-wasm-bindgen.js', import.meta.url),
)
const committedLock = fileURLToPath(
  new URL('../../../../Cargo.lock', import.meta.url),
)
const committedVersion = readFileSync(committedLock, 'utf8').match(
  /\[\[package\]\]\r?\nname = "wasm-bindgen"\r?\nversion = "([^"]+)"/,
)?.[1]
const roots: string[] = []
const packageEntry = (version: string) =>
  `[[package]]\nname = "wasm-bindgen"\nversion = ${JSON.stringify(version)}\n`

afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

describe('committed wasm-bindgen version resolution', () => {
  it.each(['0.2.128', '0.2.129', '1.0.0-rc.1+build.42'])(
    'preserves the exact committed semver when it is %s',
    (version) => {
      const lock = `version = 4\n${packageEntry(version)}`
      const result = resolveWasmBindgenVersion(lock)
      expect(result).toBe(version)
    },
  )

  it('deduplicates equal versions when package sources differ', () => {
    const lock = `${packageEntry('0.2.128')}source = "registry+fixture"\n${packageEntry('0.2.128')}source = "git+fixture"\n`
    const result = resolveWasmBindgenVersion(lock)
    expect(result).toBe('0.2.128')
  })

  it('ignores other packages and misleading comments when resolving', () => {
    const lock = `# name = "wasm-bindgen"\n[[package]]\nname = "wasm-bindgen-cli"\nversion = "9.0.0"\n${packageEntry('0.2.128')}`
    const result = resolveWasmBindgenVersion(lock)
    expect(result).toBe('0.2.128')
  })

  it.each([
    { lock: '' },
    { lock: 'version = 4' },
    { lock: '[package]\nname = "wasm-bindgen"\nversion = "0.2.128"' },
    { lock: 'package = ["wasm-bindgen", 128]' },
    { lock: '[[package]]\nname = "other"\nversion = "0.2.128"' },
    { lock: '[[package]]\nname = "wasm-bindgen"' },
    { lock: '[[package]]\nname = "wasm-bindgen"\nversion = 128' },
    { lock: packageEntry('0.2.128') + packageEntry('0.2.129') },
    { lock: packageEntry('0.2.128') + packageEntry('invalid') },
    { lock: '[[package]\nname = "wasm-bindgen"' },
    { lock: '[[package]]\nname = "wasm-bindgen"\nversion = "unterminated' },
  ])('rejects invalid, missing or ambiguous lock packages %j', ({ lock }) => {
    expect(() => resolveWasmBindgenVersion(lock)).toThrow()
  })

  it.each([
    '',
    'latest',
    'v0.2.128',
    '^0.2.128',
    '0.2',
    '00.2.128',
    '0.02.128',
    '0.2.0128',
    '0.2.128-01',
    '0.2.128+',
    '0.2.128-',
    ' 0.2.128',
    '0.2.128\n',
  ])('rejects an invalid committed version when it is %j', (version) => {
    const lock = packageEntry(version)
    expect(() => resolveWasmBindgenVersion(lock)).toThrow()
  })

  it('resolves the real committed Cargo.lock when used as library input', () => {
    const lock = readFileSync(committedLock, 'utf8')
    const version = resolveWasmBindgenVersion(lock)
    expect(committedVersion).toBeDefined()
    expect(version).toBe(committedVersion)
  })
})

describe('wasm-bindgen resolver CLI', () => {
  it('prints only the locked version when given the real Cargo.lock', () => {
    const result = spawnSync(process.execPath, [resolver, committedLock], {
      encoding: 'utf8',
    })
    expect(result.status).toBe(0)
    expect(committedVersion).toBeDefined()
    expect(result.stdout).toBe(`${committedVersion}\n`)
    expect(result.stderr).toBe('')
  })

  it.each([
    { args: [] },
    { args: ['Cargo.lock'] },
    { args: [committedLock, committedLock] },
    { args: [join(committedLock, 'absent')] },
  ])(
    'fails without version output when arguments are invalid %j',
    ({ args }) => {
      const result = spawnSync(process.execPath, [resolver, ...args], {
        encoding: 'utf8',
      })
      expect(result.status).toBe(1)
      expect(result.stdout).toBe('')
      expect(result.stderr).not.toBe('')
    },
  )

  it.each(['version = 4', '[[package]', packageEntry('bad')])(
    'fails without version output when the lock is invalid %j',
    (lock) => {
      const root = mkdtempSync(join(tmpdir(), 'wasm-bindgen-version-'))
      roots.push(root)
      const file = join(root, 'Cargo.lock')
      writeFileSync(file, lock)
      const result = spawnSync(process.execPath, [resolver, file], {
        encoding: 'utf8',
      })
      expect(result.status).toBe(1)
      expect(result.stdout).toBe('')
      expect(result.stderr).not.toBe('')
    },
  )
})
