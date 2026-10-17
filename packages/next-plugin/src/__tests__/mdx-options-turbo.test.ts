import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'
import { runInNewContext } from 'node:vm'

import { expect, it } from 'bun:test'

import {
  createMdxOptionsInstance,
  MdxTurboOptionsError,
} from '../mdx-options-instance'
import { isMdxRecord, type MdxLoader } from '../mdx-pipeline'
import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

const installed = createRequire(
  join(resolve(import.meta.dir, '../../../../apps/landing'), 'package.json'),
)
const source = readFileSync(installed.resolve('next/dist/build/swc'), 'utf8')
const start = source.indexOf('function serializeTurbopackRules(')
const end = source.indexOf('function napiEntrypointsToRawEntrypoints(', start)
const nativeSerialize: unknown = runInNewContext(
  `(${source.slice(start, end).trim()})`,
  { require: installed, JSON, Error },
)
if (typeof nativeSerialize !== 'function')
  throw new TypeError('installed serializer unavailable')

const nonJson = [
  { label: 'function', value: () => undefined },
  { label: 'undefined', value: undefined },
  { label: 'symbol', value: Symbol('option') },
  { label: 'nan', value: NaN },
  { label: 'infinity', value: Infinity },
  { label: 'negative zero', value: -0 },
  { label: 'date', value: new Date(0) },
  { label: 'regexp', value: /option/ },
  { label: 'map', value: new Map([['key', 'value']]) },
  { label: 'set', value: new Set(['value']) },
  { label: 'bigint', value: 1n },
  { label: 'sparse array', value: Array(1) },
] as const

it.each([...nonJson])(
  'rejects $label just as installed Next serialization does',
  async ({ value }) => {
    // Given
    const request = fixture('@next/mdx/mdx-js-loader', { value })
    const pipeline = { ...request.pipeline, bundler: 'turbo' as const }
    // When
    const failure = await compileMdx({ ...request, pipeline }).catch(
      (cause: unknown) => cause,
    )
    let nativeFailure: unknown
    try {
      nativeSerialize({ [pipeline.ruleKey]: { loaders: pipeline.loaders } })
    } catch (cause) {
      if (!(cause instanceof Error)) throw cause
      nativeFailure = cause
    }
    // Then
    expect(failure).toBeInstanceOf(MdxCompileError)
    if (
      !(failure instanceof MdxCompileError) ||
      !isMdxRecord(nativeFailure) ||
      typeof nativeFailure.message !== 'string'
    )
      throw new TypeError('expected located native rejection')
    expect(failure.filename).toBe(request.filename)
    expect(failure.cause instanceof Error ? failure.cause.message : '').toBe(
      nativeFailure.message,
    )
    expect(request.pipeline.loaders[0]?.options).toEqual({ value })
  },
)

it('rejects cycles with the same serialization cause before invoking a Turbo wrapper', async () => {
  // Given
  const cycle: { self?: unknown } = {}
  cycle.self = cycle
  const request = fixture('@next/mdx/mdx-js-loader', cycle)
  // When
  const error = await compileMdx({
    ...request,
    pipeline: { ...request.pipeline, bundler: 'turbo' },
  }).catch((cause: unknown) => cause)
  // Then
  expect(error).toBeInstanceOf(MdxCompileError)
  if (!(error instanceof MdxCompileError))
    throw new TypeError('expected located failure')
  expect(error.cause).toBeInstanceOf(TypeError)
  expect(cycle.self).toBe(cycle)
})

it('breaks shared JSON aliases in the worker instance while keeping the caller identities', () => {
  // Given
  const shared = { enabled: true }
  const options = { z: shared, a: shared, values: [1, null, 'text', false] }
  const request = fixture('@next/mdx/mdx-js-loader', options)
  const pipeline = { ...request.pipeline, bundler: 'turbo' as const }
  const nativeRules: unknown = nativeSerialize({
    [pipeline.ruleKey]: { loaders: pipeline.loaders },
  })
  const nativeWire: unknown = JSON.parse(JSON.stringify(nativeRules))
  // When
  const instance = createMdxOptionsInstance().loadersFor(pipeline)[0]?.options
  // Then
  if (!isMdxRecord(instance) || !isMdxRecord(nativeWire))
    throw new TypeError('missing wire options')
  expect(instance).toEqual(options)
  expect(instance.z).not.toBe(instance.a)
  expect(instance.z).not.toBe(shared)
  expect(options.z).toBe(shared)
  expect(options.a).toBe(shared)
  expect(Object.keys(instance)).toEqual(['z', 'a', 'values'])
  expect(nativeWire[pipeline.ruleKey]).toEqual({
    loaders: [{ loader: pipeline.loaders[0]?.loader, options: instance }],
  })
})

it('uses the native empty options map when a Turbo loader has no configured options', () => {
  // Given
  const request = fixture('@next/mdx/mdx-js-loader')
  const loader = { loader: request.pipeline.loaders[0]?.loader ?? '' }
  const pipeline = {
    ...request.pipeline,
    bundler: 'turbo' as const,
    loaders: [loader],
  }
  // When
  const instance = createMdxOptionsInstance().loadersFor(pipeline)
  // Then
  expect(instance).toEqual([{ ...loader, options: {} }])
  expect(loader).toEqual({ loader: loader.loader })
})

it('rejects string option maps at the native Turbo boundary rather than using webpack query parsing', () => {
  // Given
  const request = fixture('@next/mdx/mdx-js-loader')
  const loader: MdxLoader = {
    loader: request.pipeline.loaders[0]?.loader ?? '',
    options: 'jsx=true',
  }
  const pipeline = {
    ...request.pipeline,
    bundler: 'turbo' as const,
    loaders: [loader],
  }
  // When / Then
  expect(() => createMdxOptionsInstance().loadersFor(pipeline)).toThrow(
    MdxTurboOptionsError,
  )
  expect(nativeSerialize({ '*': { loaders: [loader] } })).toEqual({
    '*': { loaders: [loader] },
  })
})
