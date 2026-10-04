import { existsSync, mkdirSync } from 'node:fs'

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'
import type { NextConfig } from 'next'

import {
  findSessionOwner,
  installAfterCompileDrain,
  retainSession,
  type SessionOwner,
  SessionOwnershipError,
} from '../lifecycle'
import { createAppContext, createSession } from '../session'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()
let once: ReturnType<typeof spyOn>
let owners: SessionOwner[] = []
beforeEach(() => {
  once = spyOn(process, 'once').mockImplementation(() => process)
})
afterEach(() => {
  for (const owner of owners) owner.close()
  owners = []
  once.mockRestore()
})

function setup() {
  process.chdir(makeProject())
  const session = createSession(createAppContext({}, {}))
  mkdirSync(session.sessionDir, { recursive: true })
  const pending = Promise.withResolvers<void>()
  const coordinator = {
    ready: Promise.resolve(),
    prepared: pending.promise,
    close: mock(() => {}),
    drain: mock(() => pending.promise),
  }
  const releaseAdapter = mock(() => {})
  const fields = { session, coordinator, releaseAdapter }
  const owner = retainSession(fields)
  owners.push(owner)
  return { ...fields, owner, pending }
}

describe('session token owner', () => {
  it('rejects a second owner without replacing the live handle', () => {
    // Given: one token already owns a pending coordinator.
    const fields = setup()
    // When: a second config module tries to retain that token.
    expect(() => retainSession(fields)).toThrow(SessionOwnershipError)
    // Then: the first owner and its pending barrier remain authoritative.
    expect(findSessionOwner(fields.session.token)).toBe(fields.owner)
    expect(fields.coordinator.close).not.toHaveBeenCalled()
  })

  it('releases the adapter once while draining accepted pending work', async () => {
    // Given: preparation still owns accepted work.
    const { owner, pending, session, releaseAdapter, coordinator } = setup()
    // When: ownership drains and the accepted work settles.
    const drained = owner.drain()
    expect(findSessionOwner(session.token)).toBeUndefined()
    expect(existsSync(session.sessionDir)).toBe(true)
    pending.resolve()
    await drained
    owner.close()
    // Then: cleanup never releases a second adapter or closes early.
    expect(releaseAdapter).toHaveBeenCalledTimes(1)
    expect(coordinator.drain).toHaveBeenCalledTimes(1)
    expect(coordinator.close).toHaveBeenCalledTimes(1)
  })

  it('reserves a released token against stale exit callbacks', () => {
    // Given: an old process callback still owns its directory after release.
    const fields = setup()
    fields.owner.close()
    // When: another module tries to reacquire the old token.
    expect(() => retainSession(fields)).toThrow(SessionOwnershipError)
    // Then: the released token cannot designate a new directory owner.
    expect(findSessionOwner(fields.session.token)).toBeUndefined()
    expect(fields.releaseAdapter).toHaveBeenCalledTimes(1)
  })

  it('preserves the caller hook without draining a closed owner captured by an earlier config', async () => {
    // Given: a registered production hook whose owner later closes.
    const { owner, coordinator, session } = setup()
    const caller = mock(async () => {})
    const config: NextConfig = {
      compiler: { runAfterProductionCompile: caller },
    }
    installAfterCompileDrain(config, session.token)
    owner.close()
    // When: the old config invokes its saved hook.
    const metadata = { projectDir: '/p', distDir: '.next' }
    await config.compiler?.runAfterProductionCompile?.(metadata)
    // Then: only the caller hook runs, with its genuine metadata.
    expect(coordinator.drain).not.toHaveBeenCalled()
    expect(caller).toHaveBeenCalledWith(metadata)
  })
})
