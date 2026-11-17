import * as fs from 'node:fs'
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import {
  formatPortFile,
  isConnectionError,
  parsePortFile,
  removeOwnPortFile,
  removeStalePortFile,
  removeStaleStateDirs,
  resolveCoordinatorPortFile,
  resolveStateDir,
  unreachableCoordinatorError,
} from '../coordinator-port'

const dirs: string[] = []
function tmp(): string {
  const dir = mkdtempSync(join(tmpdir(), 'devup-port-'))
  dirs.push(dir)
  return dir
}
// A pid that cannot be alive: far above any OS limit.
const DEAD_PID = 2 ** 31 - 2
// The parent process is alive and is not this one.
const LIVE_FOREIGN_PID = process.ppid

afterEach(() => {
  for (const dir of dirs.splice(0))
    rmSync(dir, { recursive: true, force: true })
})

describe('coordinator port file', () => {
  it('keeps the port on the first line so older readers still parse it', () => {
    const text = formatPortFile(4321, 99)
    expect(Number.parseInt(text, 10)).toBe(4321)
    expect(parsePortFile(text)).toEqual({ port: 4321, pid: 99 })
  })

  it('reads a bare port written by an older release', () => {
    expect(parsePortFile('4321\n')).toEqual({ port: 4321, pid: undefined })
  })

  it('uses the shared name unless another live process owns it', () => {
    const dir = tmp()
    const shared = join(dir, 'coordinator.port')
    expect(resolveCoordinatorPortFile(dir)).toBe(shared)

    writeFileSync(shared, formatPortFile(1, DEAD_PID))
    expect(resolveCoordinatorPortFile(dir)).toBe(shared)

    writeFileSync(shared, formatPortFile(1, process.pid))
    expect(resolveCoordinatorPortFile(dir)).toBe(shared)

    writeFileSync(shared, formatPortFile(1, LIVE_FOREIGN_PID))
    expect(resolveCoordinatorPortFile(dir)).toBe(
      join(dir, `coordinator.${process.pid}.port`),
    )
  })

  it('never deletes a live foreign owner file, but clears a dead one', () => {
    const dir = tmp()
    const file = join(dir, 'coordinator.port')
    writeFileSync(file, formatPortFile(1, LIVE_FOREIGN_PID))
    removeStalePortFile(file)
    removeOwnPortFile(file)
    expect(Bun.file(file).size).toBeGreaterThan(0)

    writeFileSync(file, formatPortFile(1, DEAD_PID))
    removeStalePortFile(file)
    expect(Bun.file(file).size).toBe(0)
  })

  it('allows first-run cleanup when no coordinator port file exists', () => {
    const file = join(tmp(), 'coordinator.port')

    expect(() => removeStalePortFile(file)).not.toThrow()
  })

  it('names port, owner pid, state and recovery when the coordinator is unreachable', () => {
    const dir = tmp()
    const file = join(dir, 'coordinator.port')
    writeFileSync(file, formatPortFile(4321, DEAD_PID))
    const message = unreachableCoordinatorError(
      file,
      Object.assign(new Error('connect ETIMEDOUT 127.0.0.1:4321'), {
        code: 'ETIMEDOUT',
      }),
    ).message
    expect(message).toContain('127.0.0.1:4321')
    expect(message).toContain(`owner pid ${DEAD_PID} (not running)`)
    expect(message).toContain('restart the dev server or build')
  })

  it('writes state beside distDir only for a process with its own port file', () => {
    expect(resolveStateDir('df', join('df', 'coordinator.port'))).toBe('df')
    expect(
      resolveStateDir('df', join('df', `coordinator.${process.pid}.port`)),
    ).toBe(join('df', `state.${process.pid}`))
  })

  it('removes the state dirs of exited processes and keeps live ones', () => {
    const dir = tmp()
    for (const name of [
      `state.${DEAD_PID}`,
      `state.${process.pid}`,
      `state.${LIVE_FOREIGN_PID}`,
      'state.other',
    ])
      mkdirSync(join(dir, name))
    writeFileSync(join(dir, `state.${DEAD_PID}`, 'sheet.json'), '{}')

    removeStaleStateDirs(dir)

    expect(readdirSync(dir).sort()).toEqual(
      [
        `state.${process.pid}`,
        `state.${LIVE_FOREIGN_PID}`,
        'state.other',
      ].sort(),
    )
    // A missing distDir (first run) is nothing to clean.
    expect(() => removeStaleStateDirs(join(dir, 'missing'))).not.toThrow()
  })

  it('leaves a state dir it cannot remove for a later start', () => {
    const dir = tmp()
    mkdirSync(join(dir, `state.${DEAD_PID}`))
    const rmSpy = spyOn(fs, 'rmSync').mockImplementation(() => {
      throw Object.assign(new Error('EBUSY'), { code: 'EBUSY' })
    })
    try {
      expect(() => removeStaleStateDirs(dir)).not.toThrow()
    } finally {
      rmSpy.mockRestore()
    }
    expect(readdirSync(dir)).toEqual([`state.${DEAD_PID}`])
  })

  it('recognises connection failures, including aggregated ones', () => {
    expect(isConnectionError({ code: 'ECONNREFUSED' })).toBe(true)
    expect(isConnectionError({ errors: [{ code: 'ETIMEDOUT' }] })).toBe(true)
    expect(isConnectionError(new Error('boom'))).toBe(false)
  })
})
