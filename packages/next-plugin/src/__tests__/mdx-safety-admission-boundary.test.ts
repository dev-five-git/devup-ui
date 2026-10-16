import { expect, it } from 'bun:test'

import {
  createMdxContextAdmission,
  MdxContextRefusal,
} from '../mdx-context-admission'

it('propagates unknown parser boundary failures without manufacturing an Error message', () => {
  // Given
  const token = Symbol('parser boundary')
  const admission = createMdxContextAdmission(() => () => {
    throw token
  })
  const context = { loaders: [{ options: '{}' }] }
  const occurrence = { loader: 'boundary', index: 0, phase: 'normal' } as const
  // When
  let failure: unknown
  try {
    admission.run(context, occurrence, () => {
      const getOptions: unknown = Reflect.get(
        admission.view(context, occurrence),
        'getOptions',
      )
      if (typeof getOptions !== 'function')
        throw new TypeError('missing getOptions')
      Reflect.apply(getOptions, undefined, [])
    })
  } catch (error) {
    if (error !== token) throw error
    failure = error
  }
  // Then
  expect(failure).toBe(token)
})

it('rejects a view without original occurrence authority before reading framework getters', () => {
  // Given
  const admission = createMdxContextAdmission()
  let reads = 0
  const context = Object.defineProperty({}, 'callback', {
    get() {
      reads++
      return () => undefined
    },
  })
  // When
  const error = (() => {
    try {
      return admission.view(context, {
        loader: 'boundary',
        index: 0,
        phase: 'normal',
      })
    } catch (cause) {
      if (!(cause instanceof Error)) throw cause
      return cause
    }
  })()
  // Then
  expect(error).toBeInstanceOf(MdxContextRefusal)
  expect(reads).toBe(0)
})

it.each([
  { key: 'addDependency', context: {}, property: 'addDependency' },
  { key: 'callback', context: {}, property: 'callback' },
  { key: 'async', context: {}, property: 'async' },
  {
    key: 'async',
    context: { async: () => () => undefined, callback: () => undefined },
    property: 'async callback',
  },
  { key: 'getOptions', context: {}, property: 'getOptions occurrence' },
  {
    key: 'getOptions',
    context: { loaders: [{ options: '{}' }] },
    property: 'getOptions JSON parser unavailable',
  },
])(
  'refuses missing native capability $property rather than guessing a replacement',
  ({ key, context, property }) => {
    // Given
    const admission = createMdxContextAdmission()
    const occurrence = {
      loader: 'boundary',
      index: 0,
      phase: 'normal',
    } as const
    // When
    const error = (() => {
      try {
        return admission.run(context, occurrence, () => {
          const value: unknown = Reflect.get(
            admission.view(context, occurrence),
            key,
          )
          if (typeof value !== 'function')
            throw new TypeError('missing bounded method')
          return Reflect.apply(value, undefined, [])
        })
      } catch (cause) {
        if (!(cause instanceof Error)) throw cause
        return cause
      }
    })()
    // Then
    expect(error).toMatchObject({ property, loader: 'boundary', index: 0 })
  },
)

it('refuses unavailable descriptor storage during an admitted occurrence', () => {
  // Given
  const admission = createMdxContextAdmission()
  const context = {}
  const occurrence = { loader: 'boundary', index: 0, phase: 'pitch' } as const
  // When
  const error = (() => {
    try {
      return admission.run(context, occurrence, () =>
        Reflect.get(admission.view(context, occurrence), 'loaders'),
      )
    } catch (cause) {
      if (!(cause instanceof Error)) throw cause
      return cause
    }
  })()
  // Then
  expect(error).toMatchObject({ property: 'loaders', phase: 'pitch' })
})
