import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'
import { nativeOptionsControl, optionsProbe } from './mdx-safety-native'

it('preserves caught native malformed JSON error observations when a raw loader succeeds', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const observations: { readonly name: string; readonly message: string }[] = []
  const loader = optionsProbe(request, (getOptions) => {
    try {
      getOptions()
    } catch (error) {
      if (!(error instanceof Error)) throw error
      observations.push({ name: error.name, message: error.message })
    }
  })
  const descriptor = { loader, options: '{"broken":}' }
  await nativeOptionsControl(request, descriptor)
  expect(observations[0]?.name).toBe('Error')
  expect(observations[0]?.message).toStartWith('Cannot parse string options: ')
  // When
  const prepared = await compileMdx({
    ...request,
    pipeline: {
      ...request.pipeline,
      loaders: [...request.pipeline.loaders, descriptor],
    },
  })
  // Then
  expect(prepared.source).toContain('module.exports = 0')
  expect(observations).toHaveLength(2)
  expect(observations[1]).toEqual(observations[0])
})

it('preserves native getOptions value identity when raw options contain opaque leaves', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const options = { opaque: new Map([['key', 'value']]), callback: () => 1 }
  const values: unknown[] = []
  const loader = optionsProbe(request, (getOptions) =>
    values.push(getOptions()),
  )
  const descriptor = { loader, options }
  await nativeOptionsControl(request, descriptor)
  // When
  await compileMdx({
    ...request,
    pipeline: {
      ...request.pipeline,
      loaders: [...request.pipeline.loaders, descriptor],
    },
  })
  // Then
  expect(values).toHaveLength(2)
  expect(values[0]).toBe(options)
  expect(values[1]).toBe(options)
})

it('returns the native empty object when raw loader options are absent', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const values: unknown[] = []
  const loader = optionsProbe(request, (getOptions) =>
    values.push(getOptions()),
  )
  await nativeOptionsControl(request, { loader })
  expect(values).toEqual([{}])
  // When
  await compileMdx({
    ...request,
    pipeline: {
      ...request.pipeline,
      loaders: [...request.pipeline.loaders, { loader }],
    },
  })
  // Then
  expect(values).toEqual([{}, {}])
})

it('stickily refuses schema use even when a loader catches it after native validation fails', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const errors: unknown[] = []
  const schema = {
    title: 'SafetyLoader options',
    type: 'object',
    properties: { enabled: { type: 'boolean' } },
    additionalProperties: false,
  }
  const loader = optionsProbe(request, (getOptions) => {
    try {
      getOptions(schema)
      errors.push(undefined)
    } catch (error) {
      if (!(error instanceof Error)) throw error
      errors.push(error)
    }
  })
  const descriptor = { loader, options: { enabled: 'not-a-boolean' } }
  await nativeOptionsControl(request, descriptor)
  expect(errors[0]).toBeInstanceOf(Error)
  // When
  const result = await compileMdx({
    ...request,
    pipeline: {
      ...request.pipeline,
      loaders: [...request.pipeline.loaders, descriptor],
    },
  }).catch((error: unknown) => error)
  // Then
  const [native, prepared] = errors
  expect(prepared).toBeInstanceOf(Error)
  if (!(native instanceof Error) || !(prepared instanceof Error))
    throw new TypeError('expected both schema validations to fail')
  expect(result).toBeInstanceOf(MdxCompileError)
  expect(prepared).toMatchObject({ property: 'getOptions(schema)' })
})

it('retains all 1002 querystring keys as the webpack-created getOptions context does', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const values: unknown[] = []
  const loader = optionsProbe(request, (getOptions) =>
    values.push(getOptions()),
  )
  const options = Array.from(
    { length: 1002 },
    (_, index) => `k${index}=v${index}`,
  ).join('&')
  const descriptor = { loader, options }
  await nativeOptionsControl(request, descriptor)
  const expected = Object.fromEntries(
    Array.from({ length: 1002 }, (_, index) => [`k${index}`, `v${index}`]),
  )
  expect(values).toEqual([expected])
  // When
  await compileMdx({
    ...request,
    pipeline: {
      ...request.pipeline,
      loaders: [...request.pipeline.loaders, descriptor],
    },
  })
  // Then
  expect(values[1]).toEqual(expected)
})

it.each(['{"valid":true}', '?key=value'])(
  'preserves native string options parsing for %s',
  async (options) => {
    // Given
    const request = fixture('@mdx-js/loader')
    const values: unknown[] = []
    const loader = optionsProbe(request, (getOptions) =>
      values.push(getOptions()),
    )
    const descriptor = { loader, options }
    await nativeOptionsControl(request, descriptor)
    // When
    await compileMdx({
      ...request,
      pipeline: {
        ...request.pipeline,
        loaders: [...request.pipeline.loaders, descriptor],
      },
    })
    // Then
    expect(values).toHaveLength(2)
    expect(values[1]).toEqual(values[0])
    expect(values[0]).toEqual(
      options.startsWith('{')
        ? {
            valid: true,
            [Symbol.for('indent')]: '',
            [Symbol.for('newline')]: '',
          }
        : { '?key': 'value' },
    )
  },
)

it('returns a fresh nullish options object on each native and prepared call', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const values: unknown[] = []
  const loader = optionsProbe(request, (getOptions) => {
    values.push(getOptions(), getOptions())
  })
  await nativeOptionsControl(request, { loader })
  // When
  await compileMdx({
    ...request,
    pipeline: {
      ...request.pipeline,
      loaders: [...request.pipeline.loaders, { loader }],
    },
  })
  // Then
  expect(values).toEqual([{}, {}, {}, {}])
  expect(values[0]).not.toBe(values[1])
  expect(values[2]).not.toBe(values[3])
})
