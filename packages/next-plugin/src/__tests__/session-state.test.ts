import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterAll, describe, expect, it, spyOn } from 'bun:test'

import {
  readSessionState,
  SESSION_STATE_MANIFEST,
  type SessionStateFiles,
  writeAtomic,
  writeAtomicSync,
  writeSessionState,
  writeSessionStateSync,
} from '../session-state'

const root = fs.mkdtempSync(join(tmpdir(), 'devup-ui-session-state-'))

afterAll(() => fs.rmSync(root, { recursive: true, force: true }))

let dirs = 0
function stateFiles(): SessionStateFiles {
  dirs += 1
  const dir = join(root, String(dirs))
  fs.mkdirSync(dir)
  return [
    join(dir, 'sheet.json'),
    join(dir, 'classMap.json'),
    join(dir, 'fileMap.json'),
  ]
}

function errno(code: string): NodeJS.ErrnoException {
  return Object.assign(new Error(code), { code })
}

/** What is left in the directory besides the state files */
function leftovers(file: string): string[] {
  return fs.readdirSync(join(file, '..')).filter((f) => f.endsWith('.tmp'))
}

describe('session state', () => {
  it('restores what one write wrote', async () => {
    // Given: a state written asynchronously and one written synchronously
    const files = stateFiles()
    await writeSessionState(files, ['{"s":1}', '{"c":1}', '{"f":1}'])
    const syncFiles = stateFiles()
    writeSessionStateSync(syncFiles, ['{"s":2}', '{"c":2}', '{"f":2}'])

    // Then: each reads back whole, with no temporary file left behind
    expect(readSessionState(files)).toEqual([{ s: 1 }, { c: 1 }, { f: 1 }])
    expect(readSessionState(syncFiles)).toEqual([{ s: 2 }, { c: 2 }, { f: 2 }])
    expect(leftovers(files[0])).toEqual([])
    expect(leftovers(syncFiles[0])).toEqual([])
  })

  it('rejects files that are not one write', async () => {
    // Given: a write, then a class map from another write
    const files = stateFiles()
    await writeSessionState(files, ['{"s":1}', '{"c":1}', '{"f":1}'])
    fs.writeFileSync(files[1], '{"c":2}')

    // Then: the mix is refused
    expect(readSessionState(files)).toBeNull()
  })

  it('rejects a manifest it cannot read', async () => {
    const files = stateFiles()
    await writeSessionState(files, ['{}', '{}', '{}'])
    const manifest = join(files[0], '..', SESSION_STATE_MANIFEST)

    fs.writeFileSync(manifest, '{"version":')
    expect(readSessionState(files)).toBeNull()
    fs.writeFileSync(manifest, 'null')
    expect(readSessionState(files)).toBeNull()
    fs.writeFileSync(manifest, '{"version":2,"hashes":[]}')
    expect(readSessionState(files)).toBeNull()
    fs.writeFileSync(manifest, '{"version":1}')
    expect(readSessionState(files)).toBeNull()
  })

  it('reads state an earlier release wrote without a manifest', () => {
    const files = stateFiles()
    files.forEach((file) => fs.writeFileSync(file, '{"old":true}'))
    expect(readSessionState(files)).toEqual([
      { old: true },
      { old: true },
      { old: true },
    ])
  })

  it('has nothing to restore when a file is missing or corrupt', async () => {
    const files = stateFiles()
    expect(readSessionState(files)).toBeUndefined()

    // A complete write whose content does not parse
    await writeSessionState(files, ['{', '{}', '{}'])
    expect(readSessionState(files)).toBeUndefined()
  })
})

describe('writeAtomic', () => {
  it('retries a rename Windows refuses for a moment', async () => {
    // Given: a target another process holds open twice
    const [file] = stateFiles()
    const rename = fs.rename
    let refusals = 0
    using _rename = spyOn(fs, 'rename').mockImplementation(
      (from: any, to: any, cb: any) => {
        if (refusals < 2) {
          refusals += 1
          cb(errno(refusals === 1 ? 'EPERM' : 'EBUSY'))
          return
        }
        rename(from, to, cb)
      },
    )

    // Then: the write lands once it is released
    await writeAtomic(file, 'content')
    expect(refusals).toBe(2)
    expect(fs.readFileSync(file, 'utf-8')).toBe('content')
  })

  it('gives up and removes its temporary file', async () => {
    const [file] = stateFiles()
    let attempts = 0
    using _rename = spyOn(fs, 'rename').mockImplementation(
      (_from: any, _to: any, cb: any) => {
        attempts += 1
        cb(errno('EACCES'))
      },
    )

    await expect(writeAtomic(file, 'content')).rejects.toThrow('EACCES')
    expect(attempts).toBe(10)
    expect(leftovers(file)).toEqual([])
    expect(fs.existsSync(file)).toBe(false)
  })

  it('fails when the temporary file cannot be written', async () => {
    const missing = join(root, 'missing', 'sheet.json')
    await expect(writeAtomic(missing, 'content')).rejects.toThrow()
  })

  it('retries synchronously, then gives up on other errors', () => {
    // Given: a rename that is busy once
    const [file, other] = stateFiles()
    const renameSync = fs.renameSync
    let calls = 0
    const renameSpy = spyOn(fs, 'renameSync').mockImplementation((from, to) => {
      calls += 1
      if (calls === 1) throw errno('EBUSY')
      renameSync(from, to)
    })
    try {
      writeAtomicSync(file, 'content')
      expect(calls).toBe(2)
      expect(fs.readFileSync(file, 'utf-8')).toBe('content')

      // When: the rename fails for another reason
      renameSpy.mockImplementation(() => {
        throw errno('EXDEV')
      })

      // Then: it throws at once, leaving nothing behind
      expect(() => writeAtomicSync(other, 'content')).toThrow('EXDEV')
      expect(leftovers(other)).toEqual([])
    } finally {
      renameSpy.mockRestore()
    }
  })
})
