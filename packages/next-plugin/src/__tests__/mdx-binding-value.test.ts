import { expect, it } from 'bun:test'

import { bindingReceipt } from './mdx-binding-fixture'

it.each([
  [1, 2, 'loaders[0].options'],
  [NaN, NaN, undefined],
  [0, -0, 'loaders[0].options'],
  [1n, 1n, undefined],
  [undefined, null, 'loaders[0].options'],
  [{ x: 1 }, 1, 'loaders[0].options'],
  [1, { x: 1 }, 'loaders[0].options'],
  [[1], { 0: 1, length: 1 }, 'loaders[0].options'],
  [{ x: 1 }, {}, 'loaders[0].options.x'],
  [{}, { x: 1 }, 'loaders[0].options.x'],
  [[1], [1, 2], 'loaders[0].options.length'],
  [
    { nested: { first: 1, second: 2 } },
    { nested: { first: 2, second: 3 } },
    'loaders[0].options.nested.first',
  ],
] as const)(
  'compares primitive and shell differences %#',
  (left, right, path) => {
    // Given
    const original = bindingReceipt(left)
    // When
    const difference = original.difference(bindingReceipt(right))
    // Then
    expect(difference?.path).toBe(path)
  },
)

it('accepts null-prototype structural copies preserving option key order', () => {
  // Given
  const left = bindingReceipt({ first: 1, second: 2 })
  const right: Record<string, unknown> = Object.create(null)
  right.first = 1
  right.second = 2
  // When
  const difference = left.difference(bindingReceipt(right))
  // Then
  expect(difference).toBeUndefined()
})

it('rejects option key order differences that a native plugin can observe', () => {
  // Given
  const left = bindingReceipt({ red: 1, blue: 1 })
  // When
  const difference = left.difference(bindingReceipt({ blue: 1, red: 1 }))
  // Then
  expect(difference?.path).toBe('loaders[0].options')
})

it('compares nonplain objects by identity rather than equal contents', () => {
  // Given
  const date = new Date(0)
  const left = bindingReceipt({ date })
  // When
  const difference = left.difference(bindingReceipt({ date: new Date(0) }))
  // Then
  expect(difference?.path).toBe('loaders[0].options.date')
  expect(left.difference(bindingReceipt({ date }))).toBeUndefined()
})

it('retains function identity even when JSON projections are equal', () => {
  // Given
  const left = { plugin: () => undefined }
  const right = { plugin: () => undefined }
  expect(JSON.stringify(left)).toBe(JSON.stringify(right))
  // When
  const difference = bindingReceipt(left).difference(bindingReceipt(right))
  // Then
  expect(difference?.path).toBe('loaders[0].options.plugin')
})

it('compares symbols by identity and reports symbol property paths', () => {
  // Given
  const key = Symbol('plugin')
  const left = bindingReceipt({ [key]: Symbol.for('value') })
  // When
  const difference = left.difference(bindingReceipt({ [key]: Symbol('value') }))
  // Then
  expect(difference?.path).toBe('loaders[0].options[Symbol(plugin)]')
  expect(
    left.difference(bindingReceipt({ [key]: Symbol.for('value') })),
  ).toBeUndefined()
})

it('compares cyclic copies without requiring shared shell identity', () => {
  // Given
  const left: Record<string, unknown> = { value: 1 }
  left.self = left
  const right: Record<string, unknown> = { value: 1 }
  right.self = right
  const receipt = bindingReceipt(left)
  // When
  const difference = receipt.difference(bindingReceipt(right))
  // Then
  expect(difference).toBeUndefined()
  expect(receipt.difference(bindingReceipt(left))).toBeUndefined()
})

it('finds a differing leaf past a cyclic back edge', () => {
  // Given
  const left: Record<string, unknown> = {}
  const right: Record<string, unknown> = {}
  left.self = left
  right.self = right
  left.value = 1
  right.value = 2
  // When
  const difference = bindingReceipt(left).difference(bindingReceipt(right))
  // Then
  expect(difference?.path).toBe('loaders[0].options.value')
})

it('accepts shared-versus-copied containers while detecting a second-pair difference', () => {
  // Given
  const shared = { value: 1 }
  const left = bindingReceipt([shared, shared])
  // When
  const difference = left.difference(
    bindingReceipt([{ value: 1 }, { value: 2 }]),
  )
  // Then
  expect(difference?.path).toBe('loaders[0].options[1].value')
  expect(
    left.difference(bindingReceipt([{ value: 1 }, { value: 1 }])),
  ).toBeUndefined()
})

it('snapshots accessor descriptors without invoking user getters', () => {
  // Given
  let calls = 0
  const get = () => {
    calls += 1
    return 1
  }
  const set = () => undefined
  const left = Object.defineProperty({}, 'plugin', {
    get,
    set,
    configurable: true,
  })
  const right = Object.defineProperty({}, 'plugin', {
    get,
    set,
    configurable: true,
  })
  // When
  const difference = bindingReceipt(left).difference(bindingReceipt(right))
  // Then
  expect(difference).toBeUndefined()
  expect(calls).toBe(0)
})

it.each(['get', 'set'] as const)(
  'rejects different %s accessor identity without calling it',
  (key) => {
    // Given
    const accessor = () => {
      throw new TypeError('must not call')
    }
    const left = Object.defineProperty({}, 'plugin', { [key]: accessor })
    const right = Object.defineProperty({}, 'plugin', {
      [key]: () => undefined,
    })
    // When
    const difference = bindingReceipt(left).difference(bindingReceipt(right))
    // Then
    expect(difference?.path).toBe('loaders[0].options.plugin')
  },
)

it.each([
  [{ value: 1 }, { get: () => 1 }],
  [{ get: () => 1 }, { value: 1 }],
  [{ value: 1, enumerable: true }, { value: 1 }],
])('compares descriptor boundary differences %#', (left, right) => {
  // Given
  const original = Object.defineProperty({}, 'option', left)
  const receiving = Object.defineProperty({}, 'option', right)
  // When
  const difference = bindingReceipt(original).difference(
    bindingReceipt(receiving),
  )
  // Then
  expect(difference?.path).toBe('loaders[0].options.option')
})

it('handles a disappearing proxy descriptor without reading its value', () => {
  // Given
  const options = new Proxy(
    { gone: 1 },
    { getOwnPropertyDescriptor: () => undefined },
  )
  // When
  const difference = bindingReceipt(options).difference(bindingReceipt({}))
  // Then
  expect(difference).toBeUndefined()
})

it('accepts frozen structural shells copied into mutable config containers', () => {
  // Given
  const plugin = () => undefined
  const options = Object.freeze({ plugins: Object.freeze([plugin]) })
  // When
  const difference = bindingReceipt(options).difference(
    bindingReceipt({ plugins: [plugin] }),
  )
  // Then
  expect(difference).toBeUndefined()
})
