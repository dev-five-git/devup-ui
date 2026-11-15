import { expect, it } from 'bun:test'

import {
  operation,
  snapshotEngine,
} from '../../../plugin-utils/src/__tests__/generation-fixture'
import {
  BuildGeneration,
  ClosedBuildGenerationError,
} from '../../../plugin-utils/src/build-generation'

function memoryEngine() {
  let value = ''
  return snapshotEngine({
    reset() {
      value = ''
    },
    restore(state: string) {
      value = state
    },
    capture() {
      return value
    },
    append(part: string) {
      value += part
    },
  })
}

it('restores only its own contribution when sibling generations interleave', () => {
  // Given two live owners sharing the existing test transport.
  const engine = memoryEngine()
  const first = new BuildGeneration<string>()
  const second = new BuildGeneration<string>()
  try {
    first.run(
      engine,
      operation(() => engine.append('red')),
    )
    second.run(
      engine,
      operation(() => engine.append('blue')),
    )
    // When the first owner resumes and configures before its action.
    const result = first.run(engine, {
      ...operation(() => engine.capture()),
      configure: () => engine.append(':configured'),
    })
    // Then another owner's state cannot enter the returned contribution.
    expect(result).toBe('red:configured')
  } finally {
    first.dispose()
    second.dispose()
  }
})

it('retains allocated state when an action throws before publication', () => {
  // Given a private generation and the original failure object.
  const engine = memoryEngine()
  const owner = new BuildGeneration<string>()
  const cause = new RangeError('extraction failed')
  try {
    // When an action fails after allocating state.
    expect(() =>
      owner.run(
        engine,
        operation(() => {
          engine.append('allocated')
          throw cause
        }),
      ),
    ).toThrow(cause)
    // Then the finally-captured allocation is available to a later consumer.
    expect(
      owner.run(
        engine,
        operation(() => engine.capture()),
      ),
    ).toBe('allocated')
  } finally {
    owner.dispose()
  }
})

it('starts empty when the owning generation clears its saved handoff', () => {
  // Given a captured contribution and unrelated live engine contents.
  const engine = memoryEngine()
  const owner = new BuildGeneration<string>()
  try {
    owner.run(
      engine,
      operation(() => engine.append('obsolete')),
    )
    engine.append(':outside-owner')
    // When the owner invalidates its handoff.
    owner.clear()
    // Then the next operation resets rather than restoring obsolete state.
    expect(
      owner.run(
        engine,
        operation(() => engine.capture()),
      ),
    ).toBe('')
  } finally {
    owner.dispose()
  }
})

it('waits for the remaining participant when terminal completion arrives first', () => {
  // Given a bound generation with a terminal client and a live server.
  const engine = memoryEngine()
  const owner = new BuildGeneration<string>()
  const serverClose = owner.acquire(false)
  const clientClose = owner.acquire(true)
  try {
    owner.run(
      engine,
      operation(() => engine.append('shared')),
    )
    // When the client closes twice while the server still participates.
    clientClose()
    clientClose()
    // Then duplicate release cannot prematurely dispose the server's handoff.
    expect(owner.disposed).toBe(false)
    expect(
      owner.run(
        engine,
        operation(() => engine.capture()),
      ),
    ).toBe('shared')
    serverClose()
    expect(owner.disposed).toBe(true)
    expect(() => owner.acquire(false)).toThrow(ClosedBuildGenerationError)
    expect(() =>
      owner.run(
        engine,
        operation(() => 'unreachable'),
      ),
    ).toThrow(ClosedBuildGenerationError)
  } finally {
    clientClose()
    serverClose()
    owner.dispose()
  }
})

it('does not capture a disposed handoff when its action aborts setup', () => {
  // Given a capture observer on the existing transport, not a replacement owner.
  let captures = 0
  const engine = snapshotEngine({
    reset: () => undefined,
    restore: (_state: string) => undefined,
    capture() {
      captures += 1
      return 'checkpoint'
    },
  })
  const owner = new BuildGeneration<string>()
  try {
    // When disposal occurs inside the action's finally-protected region.
    owner.run(
      engine,
      operation(() => owner.dispose()),
    )
    // Then cleanup cannot republish disposed state.
    expect(captures).toBe(0)
    expect(() =>
      owner.run(
        engine,
        operation(() => 'unreachable'),
      ),
    ).toThrow(ClosedBuildGenerationError)
  } finally {
    owner.dispose()
  }
})

it('rejects execution when an owner closed before binding an engine', () => {
  // Given an owner closed before any operation or handle allocation.
  const owner = new BuildGeneration<string>()
  const engine = memoryEngine()
  owner.dispose()
  // When the first run attempts to bind a handle after closure.
  // Then the newly bound handle is disposed before its action can execute.
  expect(() =>
    owner.run(
      engine,
      operation(() => engine.append('forbidden')),
    ),
  ).toThrow(ClosedBuildGenerationError)
  expect(engine.capture()).toBe('')
})

it('rejects a foreign opaque handle when the shared fixture receives one', () => {
  // Given a transport that never issued the supplied handle.
  const engine = memoryEngine()
  const foreign = Object.freeze({ disposed: false })
  // When / Then foreign identity is rejected before reset or action.
  engine.append('untouched')
  expect(() =>
    engine.ownership.run(
      foreign,
      operation(() => engine.reset()),
    ),
  ).toThrow(TypeError)
  expect(engine.capture()).toBe('untouched')
})
