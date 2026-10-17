import { isMdxRecord } from './mdx-pipeline'

export function createMdxDescriptorView(
  loaders: readonly unknown[],
  access: {
    readonly refuse: (property: PropertyKey) => never
    readonly guard: (property: PropertyKey) => void
    readonly capability: (
      name: string,
      method: (...args: readonly unknown[]) => unknown,
    ) => (...args: readonly unknown[]) => unknown
  },
) {
  const { refuse, guard, capability } = access
  const fields = new Set([
    'path',
    'query',
    'fragment',
    'options',
    'ident',
    'type',
    'request',
    'raw',
    'pitchExecuted',
    'normalExecuted',
  ])
  const slots = loaders.map(
    (slot) =>
      new Proxy(
        {},
        {
          get(_target, property) {
            guard(`loaders.${String(property)}`)
            if (
              typeof property !== 'string' ||
              !fields.has(property) ||
              !isMdxRecord(slot)
            )
              return refuse(`loaders.${String(property)}`)
            return slot[property]
          },
          has(_target, property) {
            guard(`loaders.${String(property)}`)
            if (
              typeof property !== 'string' ||
              !fields.has(property) ||
              !isMdxRecord(slot)
            )
              return refuse(`loaders.${String(property)}`)
            return Object.hasOwn(slot, property)
          },
          getOwnPropertyDescriptor(_target, property) {
            guard(`loaders.${String(property)}`)
            if (
              typeof property !== 'string' ||
              !fields.has(property) ||
              !isMdxRecord(slot)
            )
              return refuse(`loaders.${String(property)}`)
            const descriptor = Object.getOwnPropertyDescriptor(slot, property)
            if (descriptor && !('value' in descriptor))
              return refuse(`loaders.${String(property)} descriptor`)
            return descriptor
          },
          ownKeys: () => refuse('loaders.ownKeys'),
          getPrototypeOf: () => refuse('loaders.getPrototypeOf'),
          set: (_target, property) => refuse(`loaders.${String(property)}`),
          defineProperty: (_target, property) =>
            refuse(`loaders.${String(property)}`),
          deleteProperty: (_target, property) =>
            refuse(`loaders.${String(property)}`),
          setPrototypeOf: () => refuse('loaders.setPrototypeOf'),
          preventExtensions: () => refuse('loaders.preventExtensions'),
          isExtensible: () => refuse('loaders.isExtensible'),
        },
      ),
  )
  const map = capability('loaders.map', (...args) => {
    const [callback, thisArg] = args
    if (typeof callback !== 'function') return refuse('loaders.map callback')
    return slots.map((slot, index) =>
      Reflect.apply(callback, thisArg, [slot, index, collection]),
    )
  })
  const collection = new Proxy([], {
    get(_target, property) {
      guard(`loaders.${String(property)}`)
      if (property === 'length') return slots.length
      if (property === 'map') return map
      if (typeof property === 'string' && /^(0|[1-9]\d*)$/.test(property))
        return slots[Number(property)]
      return refuse(`loaders.${String(property)}`)
    },
    has(_target, property) {
      guard(`loaders.${String(property)}`)
      if (typeof property === 'string' && /^(0|[1-9]\d*)$/.test(property))
        return Number(property) < slots.length
      return refuse(`loaders.${String(property)}`)
    },
    ownKeys: () => refuse('loaders.ownKeys'),
    getOwnPropertyDescriptor: (_target, property) =>
      refuse(`loaders.${String(property)} descriptor`),
    getPrototypeOf: () => refuse('loaders.getPrototypeOf'),
    set: (_target, property) => refuse(`loaders.${String(property)}`),
    defineProperty: (_target, property) =>
      refuse(`loaders.${String(property)}`),
    deleteProperty: (_target, property) =>
      refuse(`loaders.${String(property)}`),
    setPrototypeOf: () => refuse('loaders.setPrototypeOf'),
    preventExtensions: () => refuse('loaders.preventExtensions'),
    isExtensible: () => refuse('loaders.isExtensible'),
  })
  return collection
}
