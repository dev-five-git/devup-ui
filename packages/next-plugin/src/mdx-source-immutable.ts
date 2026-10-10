import type { StaticImportGraph } from '@devup-ui/plugin-utils'

import type { MdxSourcePlan } from './mdx-source-types'
import type { SourcePlan } from './plan'

export function immutableMdxMap(
  map: string | Readonly<Record<string, unknown>> | undefined,
) {
  if (typeof map !== 'object') return map
  const copy = structuredClone(map)
  function freeze(value: unknown): void {
    if (typeof value !== 'object' || value === null || Object.isFrozen(value))
      return
    Object.freeze(value)
    for (const child of Object.values(value)) freeze(child)
  }
  freeze(copy)
  return copy
}

class ImmutableMap<K, V> implements ReadonlyMap<K, V> {
  readonly #map: Map<K, V>
  constructor(entries: Iterable<readonly [K, V]>) {
    this.#map = new Map(entries)
    Object.freeze(this)
  }
  get size() {
    return this.#map.size
  }
  get(key: K) {
    return this.#map.get(key)
  }
  has(key: K) {
    return this.#map.has(key)
  }
  entries() {
    return this.#map.entries()
  }
  keys() {
    return this.#map.keys()
  }
  values() {
    return this.#map.values()
  }
  [Symbol.iterator]() {
    return this.entries()
  }
  forEach(
    callback: (value: V, key: K, map: ReadonlyMap<K, V>) => void,
    thisArg?: unknown,
  ) {
    this.#map.forEach((value, key) => callback.call(thisArg, value, key, this))
  }
}
class ImmutableSet<T> implements ReadonlySet<T> {
  readonly #set: Set<T>
  constructor(values: Iterable<T>) {
    this.#set = new Set(values)
    Object.freeze(this)
  }
  get size() {
    return this.#set.size
  }
  has(value: T) {
    return this.#set.has(value)
  }
  entries() {
    return this.#set.entries()
  }
  keys() {
    return this.#set.keys()
  }
  values() {
    return this.#set.values()
  }
  union<U>(other: ReadonlySetLike<U>) {
    return this.#set.union(other)
  }
  intersection<U>(other: ReadonlySetLike<U>) {
    return this.#set.intersection(other)
  }
  difference<U>(other: ReadonlySetLike<U>) {
    return this.#set.difference(other)
  }
  symmetricDifference<U>(other: ReadonlySetLike<U>) {
    return this.#set.symmetricDifference(other)
  }
  isSubsetOf(other: ReadonlySetLike<unknown>) {
    return this.#set.isSubsetOf(other)
  }
  isSupersetOf(other: ReadonlySetLike<unknown>) {
    return this.#set.isSupersetOf(other)
  }
  isDisjointFrom(other: ReadonlySetLike<unknown>) {
    return this.#set.isDisjointFrom(other)
  }
  [Symbol.iterator]() {
    return this.values()
  }
  forEach(
    callback: (value: T, key: T, set: ReadonlySet<T>) => void,
    thisArg?: unknown,
  ) {
    this.#set.forEach((value) => callback.call(thisArg, value, value, this))
  }
}
function edges(map: Map<string, Set<string>>) {
  return new ImmutableMap(
    [...map].map(
      ([file, targets]) => [file, new ImmutableSet(targets)] as const,
    ),
  )
}
export function immutableMdxPlan(
  plan: SourcePlan,
  graph: StaticImportGraph,
): MdxSourcePlan {
  const fileRoutes = { ...plan.fileRoutes }
  for (const routes of Object.values(fileRoutes)) Object.freeze(routes)
  return Object.freeze({
    ...plan,
    seedFiles: Object.freeze([...plan.seedFiles]),
    expectedBaseFiles: Object.freeze([...plan.expectedBaseFiles]),
    canonicalMap: Object.freeze({ ...plan.canonicalMap }),
    fileRoutes: Object.freeze(fileRoutes),
    graph: Object.freeze({
      ...(graph.requests === undefined ? {} : { requests: graph.requests }),
      files: Object.freeze([...graph.files]),
      fileSet: new ImmutableSet(graph.fileSet),
      staticImports: edges(graph.staticImports),
      staticImporters: edges(graph.staticImporters),
      dynamicImports: edges(graph.dynamicImports),
      dynamicTargets: new ImmutableSet(graph.dynamicTargets),
      externalImports: edges(graph.externalImports ?? new Map()),
    }),
  })
}
