import { expect, it } from 'bun:test'

import {
  captureMdxBindingReceipt,
  createMdxBinding,
  MdxBindingError,
} from '../mdx-binding'
import { MdxBindingDifference } from '../mdx-binding-value'
import {
  bindingInput,
  bindingOwner,
  bindingReceipt,
} from './mdx-binding-fixture'

it('accepts the receiving pipeline when owner is unbound', () => {
  // Given
  const owner = createMdxBinding(bindingOwner)
  const receiving = bindingReceipt({ remarkPlugins: [() => undefined] })
  expect(owner.state().kind).toBe('unbound')
  // When
  const accepted = owner.accept(receiving)
  // Then
  expect(accepted).toBe(receiving)
  expect(owner.state()).toEqual({ kind: 'bound', receipt: receiving })
})

it('accepts structural config copies with the same live plugin leaves', () => {
  // Given
  const plugin = () => undefined
  const object = new URL('https://example.com')
  const owner = createMdxBinding(bindingOwner)
  const bound = owner.accept(
    bindingReceipt({
      remarkPlugins: [
        plugin,
        [plugin, { nested: [1, null, true, undefined, object] }],
      ],
    }),
  )
  // When
  const accepted = owner.accept(
    bindingReceipt({
      remarkPlugins: [
        plugin,
        [plugin, { nested: [1, null, true, undefined, object] }],
      ],
    }),
  )
  // Then
  expect(accepted).toBe(bound)
})

it('rejects a recreated phase closure at the first tuple leaf without publishing', () => {
  // Given
  const plugin = () => undefined
  const owner = createMdxBinding(bindingOwner)
  owner.accept(bindingReceipt({ remarkPlugins: [plugin, [plugin, {}]] }))
  const before = owner.state()
  const other = bindingReceipt({
    remarkPlugins: [plugin, [() => undefined, {}]],
  })
  // When
  let rejected: unknown
  try {
    owner.accept(other)
  } catch (error) {
    if (!(error instanceof MdxBindingError)) throw error
    rejected = error
  }
  // Then
  expect(rejected).toBeInstanceOf(MdxBindingError)
  if (!(rejected instanceof MdxBindingError)) throw rejected
  expect(rejected.owner).toEqual(bindingOwner)
  expect(rejected.path).toBe('loaders[0].options.remarkPlugins[1][0]')
  expect(rejected.cause).toBeInstanceOf(MdxBindingDifference)
  expect(rejected.message).toContain(`${bindingOwner.configFile}:1:1:`)
  expect(owner.state()).toBe(before)
})

it.each(['resolvedPath', 'packageVersion', 'verifiedFileHash'] as const)(
  'compares native module %s by value',
  (key) => {
    // Given
    const owner = createMdxBinding(bindingOwner)
    owner.accept(bindingReceipt({}))
    const changed = bindingReceipt({}, { [key]: 'changed' })
    // When
    const action = () => owner.accept(changed)
    // Then
    expect(action).toThrow(MdxBindingError)
    expect(bindingReceipt({}).difference(changed)?.path).toBe(
      `loaders[0].${key}`,
    )
  },
)

it.each(['sourceMap', 'layer', 'isServer', 'compilerName'])(
  'retains arbitrary user option %s as semantic',
  (key) => {
    // Given
    const owner = createMdxBinding(bindingOwner)
    owner.accept(bindingReceipt({ [key]: 'first' }))
    // When
    const action = () => owner.accept(bindingReceipt({ [key]: 'second' }))
    // Then
    expect(action).toThrow(MdxBindingError)
  },
)

it('accepts client/server delivery differences of one config evaluation', () => {
  // Given
  const owner = createMdxBinding(bindingOwner)
  const original = bindingReceipt({})
  owner.accept(original)
  const server = captureMdxBindingReceipt({
    ...bindingInput,
    loaders: original.loaders,
    delivery: {
      sourceMap: true,
      layer: 'rsc',
      isServer: true,
      compilerName: 'server',
    },
  })
  // When
  const accepted = owner.accept(server)
  // Then
  expect(accepted).toBe(original)
  expect(server.delivery).toEqual({
    sourceMap: true,
    layer: 'rsc',
    isServer: true,
    compilerName: 'server',
  })
})

it('keeps original config comparison separate from native shared option mutations', () => {
  // Given
  const plugin = () => undefined
  const shared = { count: 0 }
  const tuple: unknown[] = ['named-plugin', shared]
  const options = { remarkPlugins: [tuple] }
  const receipt = bindingReceipt(options)
  const sameOriginal = bindingReceipt({
    remarkPlugins: [['named-plugin', { count: 0 }]],
  })
  const owner = createMdxBinding(bindingOwner)
  owner.accept(receipt)
  // When native execution mutates shared options and wrapper-owned tuple shells.
  shared.count += 1
  tuple[0] = plugin
  const accepted = owner.accept(sameOriginal)
  // Then
  expect(accepted).toBe(receipt)
  expect(receipt.loaders[0]?.options).toBe(options)
  expect(shared.count).toBe(1)
  expect(receipt.difference(bindingReceipt(options))?.path).toBe(
    'loaders[0].options.remarkPlugins[0][0]',
  )
})

it('does not share bound state between local owner values', () => {
  // Given
  const first = createMdxBinding(bindingOwner)
  first.accept(bindingReceipt({ plugin: () => undefined }))
  const second = createMdxBinding(bindingOwner)
  const receiving = bindingReceipt({ plugin: () => undefined })
  // When
  const accepted = second.accept(receiving)
  // Then
  expect(accepted).toBe(receiving)
})

it('retains native loader order and opaque options for every pre-extraction step', () => {
  // Given
  const first = bindingReceipt({ raw: true }).loaders[0]
  const second = bindingReceipt('?raw', {
    resolvedPath: '/native/raw.js',
  }).loaders[0]
  if (!first || !second) throw new TypeError('missing fixture loaders')
  const receipt = captureMdxBindingReceipt({
    ...bindingInput,
    loaders: [first, second],
  })
  const swapped = captureMdxBindingReceipt({
    ...bindingInput,
    loaders: [second, first],
  })
  // When
  const difference = receipt.difference(swapped)
  // Then
  expect(difference?.path).toBe('loaders[0].resolvedPath')
  expect(receipt.loaders[1]?.options).toBe('?raw')
  expect(receipt.owner).toEqual(bindingOwner)
  expect(receipt.evaluation).toEqual(bindingInput.evaluation)
})

it.each([false, true])(
  'cannot revive a released owner when initially bound=%s',
  (bound) => {
    // Given
    const owner = createMdxBinding(bindingOwner)
    const receipt = bindingReceipt({})
    if (bound) owner.accept(receipt)
    owner.release()
    owner.release()
    // When
    const action = () => owner.accept(receipt)
    // Then
    expect(action).toThrow(MdxBindingError)
    expect(owner.state()).toEqual({ kind: 'released' })
  },
)
