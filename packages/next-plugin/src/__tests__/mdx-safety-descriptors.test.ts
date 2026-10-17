import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'
import { nativeOptionsControl } from './mdx-safety-native'

const operations = [
  'Reflect.ownKeys(value)',
  'Object.getPrototypeOf(value)',
  'Object.setPrototypeOf(value, null)',
  'Object.preventExtensions(value)',
  'Object.isExtensible(value)',
  'Object.defineProperty(value, "extra", {value: 1})',
  'Reflect.set(value, "extra", 1)',
  'Reflect.deleteProperty(value, "extra")',
  'Object.getOwnPropertyDescriptor(value, "extra")',
  '"extra" in value',
  'value.extra',
] as const

it.each(
  ['context', 'collection', 'slot', 'method'].flatMap((target) =>
    operations.map((operation) => ({ target, operation })),
  ),
)(
  'stickily refuses unsupported $target observation $operation',
  async ({ target, operation }) => {
    // Given
    const request = fixture('@mdx-js/loader')
    const raw = join(request.root, 'descriptor-probe.cjs')
    writeFileSync(
      raw,
      `module.exports = function() { const value = ${target === 'context' ? 'this' : target === 'method' ? 'this.getOptions' : target === 'slot' ? 'this.loaders[this.loaderIndex]' : 'this.loaders'}; try { void (${operation}) } catch(error) { if (!(error instanceof Error)) throw error }; return '# caught descriptor probe' }`,
    )
    // When
    const result = await compileMdx({
      ...request,
      pipeline: {
        ...request.pipeline,
        loaders: [...request.pipeline.loaders, { loader: raw }],
      },
    }).catch((error: unknown) => error)
    // Then
    expect(result).toBeInstanceOf(MdxCompileError)
    expect(result).toMatchObject({
      filename: request.filename,
      cause: { loader: raw, index: 1, phase: 'normal' },
    })
  },
)

it('matches native data descriptor observations without exposing callable slots or accessors', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const raw = join(request.root, 'descriptor-observe.cjs')
  const observations: unknown[] = []
  const options = { observe: (value: unknown) => observations.push(value) }
  writeFileSync(
    raw,
    `module.exports = function() {
    const loaders = this.loaders;
    const slot = loaders[this.loaderIndex];
    this.getOptions().observe({descriptor: Object.getOwnPropertyDescriptor(slot, 'options'), hasType: 'type' in slot, stable: loaders === this.loaders, count: loaders.length});
    return 'module.exports = 0';
  }`,
  )
  const descriptor = { loader: raw, options }
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
  const native = observations[0]
  const prepared = observations[1]
  expect(native).toMatchObject({
    descriptor: {
      value: options,
      writable: true,
      enumerable: true,
      configurable: true,
    },
    stable: true,
  })
  expect(prepared).toMatchObject({
    descriptor: {
      value: options,
      writable: true,
      enumerable: true,
      configurable: true,
    },
    stable: true,
  })
})
