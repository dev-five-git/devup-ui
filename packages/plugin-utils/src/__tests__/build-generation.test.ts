import { expect, it } from 'bun:test'

import {
  BuildGeneration,
  ClosedBuildGenerationError,
} from '../build-generation'

function memoryEngine() {
  let value = ''
  return {
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
  }
}

it('roundtrips the in-memory engine when reset and restore are used', () => {
  // Given
  const engine = memoryEngine()
  engine.append('red')
  const state = engine.capture()
  // When
  engine.reset()
  engine.restore(state)
  // Then
  expect(engine.capture()).toBe('red')
})

it('retains a contribution when related sequential participants run', () => {
  // Given
  const owner = new BuildGeneration<string>()
  const engine = memoryEngine()
  owner.run(engine, () => engine.append('red'))
  owner.acquire(false)()
  // When
  const cssOnlyConsumer = owner.run(engine, () => engine.capture())
  // Then
  expect(cssOnlyConsumer).toBe('red')
})

it('isolates live siblings when their operations interleave', () => {
  // Given
  const first = new BuildGeneration<string>()
  const second = new BuildGeneration<string>()
  const engine = memoryEngine()
  first.run(engine, () => engine.append('red'))
  second.run(engine, () => engine.append('blue'))
  // When
  const resumed = first.run(engine, () => engine.capture())
  // Then
  expect(resumed).toBe('red')
})

it('starts empty when an independent owner uses the same engine', () => {
  // Given
  const engine = memoryEngine()
  new BuildGeneration<string>().run(engine, () => engine.append('red'))
  // When
  const fresh = new BuildGeneration<string>().run(engine, () =>
    engine.capture(),
  )
  // Then
  expect(fresh).toBe('')
})

it('ignores unused configurations when the actual final participant closes', () => {
  // Given
  const owner = new BuildGeneration<string>()
  const serverClose = owner.acquire(false)
  const clientClose = owner.acquire(true)
  serverClose()
  // When
  clientClose()
  clientClose()
  // Then
  expect(owner.disposed).toBe(true)
})

it('waits for the last actual watch participant when the client closes first', () => {
  // Given
  const owner = new BuildGeneration<string>()
  const serverClose = owner.acquire(true)
  owner.acquire(true)()
  expect(owner.disposed).toBe(false)
  // When
  serverClose()
  // Then
  expect(owner.disposed).toBe(true)
})

it('remembers terminal completion when another applied participant closes later', () => {
  // Given
  const owner = new BuildGeneration<string>()
  const serverClose = owner.acquire(false)
  owner.acquire(true)()
  expect(owner.disposed).toBe(false)
  // When
  serverClose()
  // Then
  expect(owner.disposed).toBe(true)
})

it('retains allocated state when an extraction fails before sheet publication', () => {
  // Given
  const owner = new BuildGeneration<string>()
  const engine = memoryEngine()
  const cause = new TypeError('extraction fault')
  // When
  expect(() =>
    owner.run(engine, () => {
      engine.append('allocation')
      throw cause
    }),
  ).toThrow(cause)
  // Then
  expect(owner.run(engine, () => engine.capture())).toBe('allocation')
})

it('clears rather than republishes a handoff when setup aborts inside an operation', () => {
  // Given
  const owner = new BuildGeneration<string>()
  const engine = memoryEngine()
  // When
  owner.run(engine, () => owner.dispose())
  // Then
  expect(() => owner.run(engine, () => engine.capture())).toThrow(
    ClosedBuildGenerationError,
  )
})

it('rejects a new participant when an integration has disposed', () => {
  // Given
  const owner = new BuildGeneration<string>()
  owner.dispose()
  // When / Then
  expect(() => owner.acquire(false)).toThrow(ClosedBuildGenerationError)
})
