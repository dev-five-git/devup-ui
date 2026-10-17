import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { expect, it } from 'bun:test'

import type { MdxInvocationLoader } from '../mdx-invocation'
import {
  createMdxOptionsInstance,
  MdxTurboOptionsError,
} from '../mdx-options-instance'
import { isMdxRecord } from '../mdx-pipeline'
import { compileMdx } from '../mdx-prepare'
import { fullFixture } from './mdx-own-slot-fixture'

it.each(['string-request', 'module-string-options', 'caller-options-ident'])(
  'retains native descriptor form %s through full capture',
  async (form) => {
    // Given
    const request = fullFixture('return source')
    const raw = join(request.root, 'form.mjs')
    writeFileSync(
      raw,
      `export const observations = []; export const raw = true; export default function(source, map) {
    this.callback(null, source, map)
  }; export function pitch() {
    observations.push(this.loaders[this.loaderIndex].request)
  }`,
    )
    const rawModule: unknown = await import(pathToFileURL(raw).href)
    if (!isMdxRecord(rawModule) || !Array.isArray(rawModule.observations))
      throw new TypeError('missing module observations')
    const descriptor: MdxInvocationLoader =
      form === 'string-request'
        ? raw + '?caller=value#fragment'
        : {
            loader: raw,
            type: 'module',
            fragment: '#fragment',
            options:
              form === 'module-string-options'
                ? 'caller=value'
                : { caller: 'value', ident: 'caller-ident' },
          }
    const own = request.invocation.loaders[0]
    const compiler = request.invocation.loaders[1]
    if (!own || !compiler) throw new TypeError('missing original slots')
    // When
    const output = await compileMdx({
      ...request,
      invocation: {
        ...request.invocation,
        loaders: [own, compiler, descriptor],
      },
    })
    // Then
    expect(output.source).toContain('original')
    expect(rawModule.observations).toEqual([
      raw +
        (form === 'caller-options-ident' ? '??caller-ident' : '?caller=value') +
        '#fragment',
    ])
  },
)

it('applies exact Turbo serialization when the same invocation is reused under another pipeline', () => {
  // Given
  const request = fullFixture('return source')
  const [own, compiler, raw] = request.invocation.loaders
  if (!own || !compiler || !raw) throw new TypeError('missing original slots')
  const callback = () => undefined
  const invocation = {
    ...request.invocation,
    loaders: [own, compiler, { ...raw, options: { callback } }],
  }
  const instance = createMdxOptionsInstance()
  instance.invocationLoadersFor(request.pipeline, invocation)
  // When / Then
  expect(() =>
    instance.invocationLoadersFor(
      { ...request.pipeline, bundler: 'turbo' },
      invocation,
    ),
  ).toThrow(MdxTurboOptionsError)
})

it('preserves full Turbo string requests and descriptor metadata while applying native JSON options', () => {
  // Given
  const request = fullFixture('return source')
  const [own, compiler, raw] = request.invocation.loaders
  if (!own || !compiler || !raw) throw new TypeError('missing original slots')
  const invocation = {
    ...request.invocation,
    loaders: [
      own,
      { ...compiler, ident: 'caller', fragment: '#fragment', type: 'commonjs' },
      raw.loader + '?original=query#raw',
    ],
  }
  // When
  const loaders = createMdxOptionsInstance().invocationLoadersFor(
    { ...request.pipeline, bundler: 'turbo' },
    invocation,
  )
  // Then
  expect(loaders[1]).toEqual(invocation.loaders[1])
  expect(loaders[2]).toBe(invocation.loaders[2])
})

it.each(['webpack', 'turbo'] as const)(
  'reuses only run-local full-invocation options for %s',
  (bundler) => {
    // Given
    const request = fullFixture('return source')
    const pipeline = { ...request.pipeline, bundler }
    const instance = createMdxOptionsInstance()
    // When
    const first = instance.invocationLoadersFor(pipeline, request.invocation)
    // Then
    expect(instance.invocationLoadersFor(pipeline, request.invocation)).toBe(
      first,
    )
    expect(
      createMdxOptionsInstance().invocationLoadersFor(
        pipeline,
        request.invocation,
      ),
    ).not.toBe(first)
  },
)

it('detaches a compiler options root once per run while preserving each original descriptor shell', () => {
  // Given
  const request = fullFixture('return source')
  const compiler = request.invocation.loaders[1]
  if (!compiler) throw new TypeError('missing original compiler')
  const invocation = {
    ...request.invocation,
    loaders: request.invocation.loaders.map((loader, index) =>
      index === 1 ? { ...loader, fragment: '#resource-local' } : loader,
    ),
  }
  const instance = createMdxOptionsInstance()
  const first = instance.invocationLoadersFor(
    request.pipeline,
    request.invocation,
  )[1]
  // When
  const second = instance.invocationLoadersFor(request.pipeline, invocation)[1]
  // Then
  if (
    !first ||
    typeof first === 'string' ||
    !second ||
    typeof second === 'string'
  )
    throw new TypeError('missing compiler option shells')
  expect(first.options).toBe(second.options)
  expect(first.options).not.toBe(compiler.options)
  expect(second.fragment).toBe('#resource-local')
})
