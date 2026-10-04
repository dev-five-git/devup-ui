import { randomUUID } from 'node:crypto'
import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import {
  foreignLiveOwner,
  formatPortFile,
  isConnectionError,
  isProcessAlive,
  missingPortFileError,
  parsePortFile,
  publishPortFile,
  readOwner,
  removeOwnPortFile,
  removeStalePortFile,
  resolveCoordinatorPortFile,
  unreachableCoordinatorError,
} from '../coordinator-port'

const dirs: string[] = []
const identity = { project: resolve('.'), token: randomUUID() }
const deadPid = 2 ** 31 - 2
function fixture() {
  const dir = fs.mkdtempSync(join(tmpdir(), 'devup-port-'))
  dirs.push(dir)
  return join(dir, 'coordinator.port')
}
afterEach(() => {
  for (const dir of dirs.splice(0))
    fs.rmSync(dir, { recursive: true, force: true })
})

describe('coordinator descriptors', () => {
  it('roundtrips strict ownership when formatted', () => {
    const text = formatPortFile(4321, 99, identity)
    expect(parsePortFile(text)).toEqual({
      version: 1,
      port: 4321,
      pid: 99,
      ...identity,
    })
    expect(parsePortFile(formatPortFile(1)).pid).toBe(process.pid)
  })
  it.each([
    { version: 2 },
    { port: 0 },
    { port: 65536 },
    { port: 1.5 },
    { port: '1' },
    { pid: 0 },
    { pid: 1.5 },
    { pid: '1' },
    { project: '' },
    { project: '.' },
    { project: `${resolve('.')}/..` },
    { token: 'bad' },
  ])('rejects malformed ownership %j', (override) => {
    expect(() =>
      parsePortFile(
        JSON.stringify({
          version: 1,
          port: 1,
          pid: 1,
          ...identity,
          ...override,
        }),
      ),
    ).toThrow()
  })
  it.each([
    'null',
    '[]',
    '{}',
    '1234',
    '{',
    ' '.repeat(4097),
    JSON.stringify({
      version: 1,
      port: 1,
      pid: 1,
      ...identity,
      padding: '한'.repeat(1400),
    }),
  ])('rejects invalid or oversized metadata', (text) => {
    expect(() => parsePortFile(text)).toThrow()
  })
  it('atomically replaces a descriptor when published', () => {
    const file = fixture()
    fs.writeFileSync(file, 'old')
    const info = parsePortFile(formatPortFile(1234, process.pid, identity))
    publishPortFile(file, info)
    expect(readOwner(file)).toEqual(info)
    expect(fs.readdirSync(join(file, '..'))).toEqual(['coordinator.port'])
  })
  it('retains previous ownership and cleans temporary files when rename fails', () => {
    const file = fixture()
    fs.writeFileSync(file, 'old')
    const rename = spyOn(fs, 'renameSync').mockImplementation(() => {
      throw new Error('rename failed')
    })
    try {
      expect(() =>
        publishPortFile(file, parsePortFile(formatPortFile(1))),
      ).toThrow('rename failed')
    } finally {
      rename.mockRestore()
    }
    expect(fs.readFileSync(file, 'utf-8')).toBe('old')
    expect(fs.readdirSync(join(file, '..'))).toEqual(['coordinator.port'])
  })
  it('preserves foreign and malformed files when removing own ownership', () => {
    const file = fixture()
    fs.writeFileSync(file, formatPortFile(1, process.ppid, identity))
    removeOwnPortFile(file, identity)
    removeStalePortFile(file)
    expect(foreignLiveOwner(file)).toBe(process.ppid)
    expect(resolveCoordinatorPortFile(join(file, '..'))).toBe(
      join(file, '..', `coordinator.${process.pid}.port`),
    )
    fs.writeFileSync(file, 'broken')
    removeOwnPortFile(file)
    expect(fs.readFileSync(file, 'utf-8')).toBe('broken')
  })
  it('removes only the current matching identity when closing', () => {
    const file = fixture()
    fs.writeFileSync(file, formatPortFile(1, process.pid, identity))
    removeOwnPortFile(file, { ...identity, token: randomUUID() })
    expect(readOwner(file)?.token).toBe(identity.token)
    removeOwnPortFile(file, { ...identity, project: resolve('other') })
    expect(readOwner(file)?.project).toBe(identity.project)
    removeOwnPortFile(file, identity)
    expect(readOwner(file)).toBeUndefined()
    removeStalePortFile(file)
  })
  it('uses shared paths when ownership is absent, dead or ours', () => {
    const file = fixture()
    expect(resolveCoordinatorPortFile(join(file, '..'))).toBe(file)
    for (const pid of [deadPid, process.pid]) {
      fs.writeFileSync(file, formatPortFile(1, pid))
      expect(resolveCoordinatorPortFile(join(file, '..'))).toBe(file)
    }
    removeStalePortFile(file)
    expect(readOwner(file)).toBeUndefined()
  })
  it('treats permission-denied process probes as alive', () => {
    const kill = spyOn(process, 'kill').mockImplementation(() => {
      throw Object.assign(new Error('denied'), { code: 'EPERM' })
    })
    try {
      expect(isProcessAlive(99)).toBe(true)
    } finally {
      kill.mockRestore()
    }
  })
  it('reports endpoint ownership and recovery for unreachable endpoints', () => {
    const file = fixture()
    expect(missingPortFileError(file).message).toContain(file)
    expect(unreachableCoordinatorError(file, 'failed').message).toContain(
      'unknown owner',
    )
    for (const pid of [deadPid, process.pid]) {
      fs.writeFileSync(file, formatPortFile(4321, pid))
      expect(
        unreachableCoordinatorError(file, new Error('failed')).message,
      ).toContain(`owner pid ${pid}`)
    }
  })
  it('recognizes connection codes without untyped assertions', () => {
    for (const error of [
      { code: 'ECONNREFUSED' },
      { errors: [{ code: 'ETIMEDOUT' }] },
    ])
      expect(isConnectionError(error)).toBe(true)
    for (const error of [
      null,
      undefined,
      'oops',
      {},
      { errors: [] },
      new Error('oops'),
    ])
      expect(isConnectionError(error)).toBe(false)
  })
})
