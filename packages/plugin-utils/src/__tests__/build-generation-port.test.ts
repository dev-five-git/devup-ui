import { expect, it } from 'bun:test'

import { BuildGeneration, type GenerationEngine } from '../build-generation'
import { beginBuild } from '../build-session'
import { operation } from './generation-fixture'

it('delegates the exact request and defers capture to the required binding port', () => {
  // Given
  const owner = new BuildGeneration<string>()
  const handle = Object.freeze({ disposed: false })
  const trace: string[] = []
  let capture: (() => string) | undefined
  const request = {
    ...operation(() => {
      trace.push('action')
      return 'result'
    }),
    configure: () => {
      trace.push('configure')
    },
  }
  const engine: GenerationEngine<string> = {
    ownership: {
      create(callback) {
        capture = callback
        return handle
      },
      run(actual, supplied) {
        expect(actual).toBe(handle)
        expect(Object.is(supplied, request)).toBe(true)
        supplied.configure()
        return supplied.action()
      },
      dispose() {
        trace.push('dispose')
      },
      clear() {
        trace.push('clear')
      },
    },
    capture() {
      trace.push('capture')
      return 'checkpoint'
    },
  }
  // When
  const result = owner.run(engine, request)
  // Then
  expect(result).toBe('result')
  expect(trace).toEqual(['configure', 'action'])
  expect(capture?.()).toBe('checkpoint')
  expect(trace).toEqual(['configure', 'action', 'capture'])
  owner.dispose()
})

it('retains legacy admission when disposal asks for owner-targeted clear', () => {
  // Given
  const owner = new BuildGeneration<string>()
  const trace: string[] = []
  const engine: GenerationEngine<string> = {
    ownership: {
      create() {
        return Object.freeze({ disposed: false })
      },
      run(_handle, request) {
        request.configure()
        return request.action()
      },
      dispose() {
        trace.push('dispose')
      },
      clear() {
        trace.push('clear')
      },
    },
    capture() {
      return 'checkpoint'
    },
  }
  owner.run(
    engine,
    operation(() => undefined),
  )
  const release = beginBuild({}, { integration: 'legacy test', root: 'legacy' })
  // When
  try {
    owner.dispose()
  } finally {
    release()
  }
  // Then
  expect(trace).toEqual(['dispose'])
  owner.clear()
  expect(trace).toEqual(['dispose', 'clear'])
})
