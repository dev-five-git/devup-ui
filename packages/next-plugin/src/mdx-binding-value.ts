type Entry = {
  readonly key: string | symbol
  readonly enumerable: boolean | undefined
} & (
  | {
      readonly kind: 'data'
      readonly value: Value
    }
  | { readonly kind: 'accessor'; readonly get: unknown; readonly set: unknown }
)
type Value =
  | { readonly kind: 'leaf'; readonly value: unknown }
  | {
      readonly kind: 'shell'
      readonly array: boolean
      readonly entries: readonly Entry[]
    }

export class MdxBindingDifference extends Error {
  readonly name = 'MdxBindingDifference'
  constructor(
    readonly path: string,
    readonly reason: string,
  ) {
    super(`${path}: ${reason}`)
  }
}

export function snapshotMdxBindingValue(input: unknown): Value {
  const seen = new Map<object, Value>()
  function snapshot(value: unknown): Value {
    if (typeof value !== 'object' || value === null)
      return { kind: 'leaf', value }
    const array = Array.isArray(value)
    const prototype: unknown = Object.getPrototypeOf(value)
    if (!array && prototype !== Object.prototype && prototype !== null)
      return { kind: 'leaf', value }
    const existing = seen.get(value)
    if (existing) return existing
    // Register before descending: cyclic shells are structural, not recursion errors.
    const entries: Entry[] = []
    const result: Value = { kind: 'shell', array, entries }
    seen.set(value, result)
    for (const key of Reflect.ownKeys(value)) {
      const descriptor = Object.getOwnPropertyDescriptor(value, key)
      if (!descriptor) continue
      const flags = {
        key,
        enumerable: descriptor.enumerable,
      }
      if ('value' in descriptor) {
        const child: unknown = descriptor.value
        entries.push({
          ...flags,
          kind: 'data',
          value: snapshot(child),
        })
      } else {
        entries.push({
          ...flags,
          kind: 'accessor',
          get: descriptor.get,
          set: descriptor.set,
        })
      }
    }
    Object.freeze(entries)
    return Object.freeze(result)
  }
  return snapshot(input)
}

function keyPath(path: string, key: string | symbol): string {
  if (typeof key === 'symbol') return `${path}[${String(key)}]`
  return /^\d+$/.test(key) ? `${path}[${key}]` : `${path}.${key}`
}

export function compareMdxBindingValues(
  left: Value,
  right: Value,
): MdxBindingDifference | undefined {
  const pairs = new Map<Value, Set<Value>>()
  function compare(
    a: Value,
    b: Value,
    path: string,
  ): MdxBindingDifference | undefined {
    switch (a.kind) {
      case 'leaf':
        if (b.kind !== 'leaf')
          return new MdxBindingDifference(path, 'value shape changed')
        return Object.is(a.value, b.value)
          ? undefined
          : new MdxBindingDifference(
              path,
              'value or live leaf identity changed',
            )
      case 'shell': {
        if (b.kind !== 'shell')
          return new MdxBindingDifference(path, 'value shape changed')
        if (a.array !== b.array)
          return new MdxBindingDifference(path, 'array/object shape changed')
        let matches = pairs.get(a)
        if (matches?.has(b)) return
        if (!matches) pairs.set(a, (matches = new Set()))
        matches.add(b)
        for (const entry of a.entries) {
          const next = b.entries.find(
            (candidate) => candidate.key === entry.key,
          )
          const childPath = keyPath(path, entry.key)
          if (!next)
            return new MdxBindingDifference(childPath, 'property removed')
          if (entry.enumerable !== next.enumerable)
            return new MdxBindingDifference(
              childPath,
              'property descriptor changed',
            )
          switch (entry.kind) {
            case 'accessor':
              if (next.kind !== 'accessor')
                return new MdxBindingDifference(
                  childPath,
                  'property descriptor changed',
                )
              if (entry.get !== next.get || entry.set !== next.set)
                return new MdxBindingDifference(
                  childPath,
                  'accessor identity changed',
                )
              break
            case 'data': {
              if (next.kind !== 'data')
                return new MdxBindingDifference(
                  childPath,
                  'property descriptor changed',
                )
              const difference = compare(entry.value, next.value, childPath)
              if (difference) return difference
              break
            }
            default:
              return entry satisfies never
          }
        }
        const added = b.entries.find(
          (entry) =>
            !a.entries.some((candidate) => candidate.key === entry.key),
        )
        if (added)
          return new MdxBindingDifference(
            keyPath(path, added.key),
            'property added',
          )
        if (
          a.entries.some((entry, index) => entry.key !== b.entries[index]?.key)
        )
          return new MdxBindingDifference(path, 'own-key order changed')
        return
      }
      default:
        return a satisfies never
    }
  }
  return compare(left, right, 'loaders')
}
