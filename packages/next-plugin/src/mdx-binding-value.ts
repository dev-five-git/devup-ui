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

export function reusableMdxBindingValue(
  snapshot: Value,
  webpackCompiler = false,
): boolean {
  type Coverage = 'none' | 'loader' | 'json'
  const seen = {
    none: new Set<Value>(),
    loader: new Set<Value>(),
    json: new Set<Value>(),
  }
  function reusable(value: Value, coverage: Coverage): boolean {
    if (seen[coverage].has(value)) return true
    seen[coverage].add(value)
    switch (value.kind) {
      case 'leaf':
        // Native current H proves timestamps only on live serialized paths.
        // Other opaque objects retain identity but do not prove compiled bytes.
        return (
          typeof value.value !== 'object' ||
          value.value === null ||
          (coverage === 'json' &&
            value.value instanceof Date &&
            Reflect.ownKeys(value.value).length === 0)
        )
      case 'shell':
        return value.entries.every((entry) => {
          switch (entry.kind) {
            case 'data': {
              let child: Coverage = 'none'
              switch (coverage) {
                case 'loader':
                  // webpackCompiler copies shells, not their nested values.
                  if (
                    entry.key === 'options' &&
                    entry.value.kind === 'shell' &&
                    !entry.value.array
                  )
                    child = 'json'
                  break
                case 'json':
                  // H uses enumerable string keys; JSON arrays use indices,
                  // not extra fields (including numeric-looking nonindices).
                  if (
                    entry.enumerable &&
                    typeof entry.key === 'string' &&
                    (!value.array ||
                      (/^(0|[1-9]\d*)$/.test(entry.key) &&
                        Number(entry.key) < 4_294_967_295))
                  )
                    child = 'json'
                  break
                case 'none':
                  break
                default:
                  coverage satisfies never
              }
              return reusable(entry.value, child)
            }
            case 'accessor':
              return false
            default:
              return entry satisfies never
          }
        })
      default:
        return value satisfies never
    }
  }
  return reusable(snapshot, webpackCompiler ? 'loader' : 'none')
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
