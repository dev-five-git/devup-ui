import { expect, it } from 'bun:test'

import { BuildGeneration } from '../build-generation'

function traceEngine() {
  const trace: string[] = []
  let value = 'initial'
  let fault: 'reset' | 'restore' | 'capture' | undefined
  const cause = new TypeError('engine fault')
  return {
    trace,
    cause,
    fail(next: typeof fault) {
      fault = next
    },
    reset() {
      trace.push('reset')
      if (fault === 'reset') throw cause
      value = 'empty'
    },
    restore(state: string) {
      trace.push(`restore:${state}`)
      if (fault === 'restore') throw cause
      value = state
    },
    capture() {
      trace.push('capture')
      if (fault === 'capture') throw cause
      return value
    },
    action(next = 'allocated') {
      trace.push('action')
      value = next
    },
  }
}

it.each(['reset', 'restore'] as const)(
  'does not enter action or finally when %s fails',
  (fault) => {
    // Given a previously captured private owner snapshot.
    const owner = new BuildGeneration<string>()
    const engine = traceEngine()
    owner.run(engine, () => engine.action())
    engine.trace.length = 0
    engine.fail(fault)
    // When reset or restore fails outside the action region.
    expect(() => owner.run(engine, () => engine.action())).toThrow(engine.cause)
    // Then no action or capture is attempted.
    expect(engine.trace).toEqual(
      fault === 'reset' ? ['reset'] : ['reset', 'restore:allocated'],
    )
  },
)

it('captures allocations when an action throws', () => {
  // Given a captured owner and a failing action.
  const owner = new BuildGeneration<string>()
  const engine = traceEngine()
  owner.run(engine, () => engine.action())
  engine.trace.length = 0
  const cause = new RangeError('action fault')
  // When action fails after mutation.
  expect(() =>
    owner.run(engine, () => {
      engine.action()
      throw cause
    }),
  ).toThrow(cause)
  // Then reset/restore still precede action and its final capture.
  expect(engine.trace).toEqual([
    'reset',
    'restore:allocated',
    'action',
    'capture',
  ])
})

it.each([false, true])(
  'keeps the previous handoff when capture fails (action fault=%s)',
  (actionFault) => {
    // Given a successful handoff and a subsequently broken capture boundary.
    const owner = new BuildGeneration<string>()
    const engine = traceEngine()
    owner.run(engine, () => engine.action())
    engine.trace.length = 0
    engine.fail('capture')
    // When capture fails, including while unwinding a different action error.
    expect(() =>
      owner.run(engine, () => {
        engine.action('uncommitted')
        if (actionFault) throw new RangeError('action fault')
      }),
    ).toThrow(engine.cause)
    // Then capture wins, while a later restore still receives the previous snapshot.
    expect(engine.trace).toEqual([
      'reset',
      'restore:allocated',
      'action',
      'capture',
    ])
    engine.fail(undefined)
    engine.trace.length = 0
    owner.run(engine, () => engine.action())
    expect(engine.trace).toEqual([
      'reset',
      'restore:allocated',
      'action',
      'capture',
    ])
  },
)

it('suppresses capture when action disposes its owner', () => {
  // Given a private owner with a saved snapshot.
  const owner = new BuildGeneration<string>()
  const engine = traceEngine()
  owner.run(engine, () => engine.action())
  engine.trace.length = 0
  // When disposal occurs within the action region.
  owner.run(engine, () => {
    engine.action()
    owner.dispose()
  })
  // Then finally must not republish disposed state.
  expect(engine.trace).toEqual(['reset', 'restore:allocated', 'action'])
  expect(owner.disposed).toBe(true)
})
