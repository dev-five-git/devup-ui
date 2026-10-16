import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fullFixture } from './mdx-own-slot-fixture'

it('closes a thenable pitch before resource IO without evaluating its then getter twice privately', async () => {
  // Given
  let reads = 0
  const seen: unknown[] = []
  const request = fullFixture(
    'return source',
    `const options = this.getOptions(); const view = this; return {
    get then() { options.read(); return resolve => { resolve(); queueMicrotask(() => {
      try { options.observe(view.resourcePath) } catch (cause) { if (!(cause instanceof Error)) throw cause; options.observe(cause) }
    }) } }
  }`,
  )
  const [own, compiler, raw] = request.invocation.loaders
  const options = {
    read: () => {
      reads++
    },
    observe: (value: unknown) => seen.push(value),
  }
  // When
  const result = await compileMdx({
    ...request,
    invocation: {
      ...request.invocation,
      loaders: [own, compiler, { ...raw, options }],
    },
  }).catch((cause: unknown) => cause)
  // Then
  expect(result).toBeInstanceOf(MdxCompileError)
  expect(seen).toHaveLength(1)
  expect(seen[0]).toMatchObject({
    loader: raw.loader,
    index: 2,
    phase: 'pitch',
    property: 'resourcePath',
  })
  expect(reads).toBe(2)
})
