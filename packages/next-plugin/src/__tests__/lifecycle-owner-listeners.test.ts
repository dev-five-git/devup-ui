import { mkdirSync } from 'node:fs'

import { expect, it } from 'bun:test'

import { retainSession } from '../lifecycle'
import { createAppContext, createSession } from '../session'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()

function owner() {
  process.chdir(makeProject())
  const session = createSession(createAppContext({}, {}))
  mkdirSync(session.sessionDir, { recursive: true })
  let closed = 0
  const retained = retainSession({
    session,
    coordinator: {
      ready: Promise.resolve(),
      close() {
        closed += 1
      },
      drain: () => Promise.resolve(),
    },
  })
  return { retained, closed: () => closed }
}

it('releases only its own process callbacks when an owner closes explicitly', async () => {
  // Given: a foreign callback and a drained owner with two owned callbacks.
  process.on('exit', Date.now)
  const beforeExit = process.listeners('beforeExit')
  const exit = process.listeners('exit')
  const input = owner()
  await input.retained.drain()
  try {
    // When: the session closes before process shutdown.
    input.retained.close()
    // Then: neither owned callback retains the released session; foreign callbacks survive.
    expect(process.listeners('beforeExit')).toEqual(beforeExit)
    expect(process.listeners('exit')).toEqual(exit)
    expect(input.closed()).toBe(1)
  } finally {
    input.retained.close()
    process.off('exit', Date.now)
  }
})

it('leaves a second live owner registered when another owner closes', () => {
  // Given: two independently owned callbacks in the real process emitter.
  const first = owner()
  const second = owner()
  const beforeExit = process.listenerCount('beforeExit')
  const exit = process.listenerCount('exit')
  try {
    // When: only the first owner closes, twice.
    first.retained.close()
    first.retained.close()
    // Then: precisely its two listeners are gone and the other coordinator stays open.
    expect(process.listenerCount('beforeExit')).toBe(beforeExit - 1)
    expect(process.listenerCount('exit')).toBe(exit - 1)
    expect(first.closed()).toBe(1)
    expect(second.closed()).toBe(0)
  } finally {
    first.retained.close()
    second.retained.close()
  }
})
