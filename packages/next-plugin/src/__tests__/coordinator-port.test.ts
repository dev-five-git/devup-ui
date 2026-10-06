import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import {
  formatPortFile,
  isConnectionError,
  parsePortFile,
  removeOwnPortFile,
  removeStalePortFile,
  resolveCoordinatorPortFile,
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

  it('recognises connection failures, including aggregated ones', () => {
    expect(isConnectionError({ code: 'ECONNREFUSED' })).toBe(true)
    expect(isConnectionError({ errors: [{ code: 'ETIMEDOUT' }] })).toBe(true)
    expect(isConnectionError(new Error('boom'))).toBe(false)
  })
})
