import { randomUUID } from 'node:crypto'

import {
  compareMdxBindingValues,
  snapshotMdxBindingValue,
} from './mdx-binding-value'
import { isMdxRecord } from './mdx-pipeline'
import { reportedMdxInputs, sourceHash } from './mdx-source-freshness'
import { immutableMdxMap } from './mdx-source-immutable'
import type {
  MdxCompiledRecord,
  MdxPipelineSelection,
} from './mdx-source-types'

type PortableValue =
  | null
  | boolean
  | number
  | string
  | readonly PortableValue[]
  | { readonly [key: string]: PortableValue }
export type MdxCacheEntry = MdxCompiledRecord & {
  readonly identity: ReturnType<typeof snapshotMdxBindingValue>
  readonly portable: PortableValue | undefined
  readonly sourceMap: boolean | undefined
  readonly mode: string | undefined
  readonly compilerKey: string
}

function portableValue(
  value: unknown,
  seen = new Set<object>(),
): PortableValue | undefined {
  if (value === null || typeof value === 'string' || typeof value === 'boolean')
    return value
  if (typeof value === 'number')
    return Number.isFinite(value) ? value : undefined
  if (typeof value !== 'object') return undefined
  const prototype: unknown = Object.getPrototypeOf(value)
  if (
    (!Array.isArray(value) &&
      prototype !== Object.prototype &&
      prototype !== null) ||
    seen.has(value)
  )
    return undefined
  seen.add(value)
  const result: Record<string, PortableValue> = {}
  for (const key of Reflect.ownKeys(value)) {
    if (Array.isArray(value) && key === 'length') continue
    if (typeof key !== 'string') return undefined
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    if (!descriptor || !('value' in descriptor)) return undefined
    const child: unknown = descriptor.value
    const portable = portableValue(child, seen)
    if (portable === undefined) return undefined
    result[key] = portable
  }
  seen.delete(value)
  if (Array.isArray(value) && Object.keys(result).length !== value.length)
    return undefined
  return Array.isArray(value)
    ? Object.freeze(Object.values(result))
    : Object.freeze(result)
}

export function captureMdxCacheIdentity(selection: MdxPipelineSelection) {
  const portable = portableValue(selection.identity)
  return {
    identity: snapshotMdxBindingValue(selection.identity),
    portable,
    compilerKey:
      portable === undefined
        ? `live:${randomUUID()}`
        : sourceHash(JSON.stringify(portable)),
    sourceMap: selection.context.sourceMap,
    mode: selection.context.mode,
  }
}
export function compatibleMdxCache(
  entry: MdxCacheEntry,
  selection: MdxPipelineSelection,
): boolean {
  return (
    entry.sourceMap === selection.context.sourceMap &&
    entry.mode === selection.context.mode &&
    compareMdxBindingValues(
      entry.identity,
      snapshotMdxBindingValue(selection.identity),
    ) === undefined
  )
}

export function exportMdxRestartCache(
  entries: ReadonlyMap<string, MdxCacheEntry>,
): string {
  return JSON.stringify(
    [...entries].flatMap(([filename, entry]) =>
      entry.portable === undefined
        ? []
        : [
            {
              filename,
              identity: entry.portable,
              sourceMap: entry.sourceMap,
              mode: entry.mode,
              prepared: entry.prepared,
              inputs: entry.inputs,
            },
          ],
    ),
  )
}

export function importMdxRestartCache(
  serialized: string,
): ReadonlyMap<string, MdxCacheEntry> {
  const data: unknown = JSON.parse(serialized)
  if (!Array.isArray(data)) throw new TypeError('Invalid MDX restart cache')
  const entries = new Map<string, MdxCacheEntry>()
  for (const item of data) {
    const value: unknown = item
    if (
      !isMdxRecord(value) ||
      typeof value.filename !== 'string' ||
      !isMdxRecord(value.prepared) ||
      !Array.isArray(value.inputs)
    )
      throw new TypeError('Invalid MDX restart entry')
    const p = value.prepared
    if (
      typeof p.source !== 'string' ||
      p.filename !== value.filename ||
      (p.map !== undefined && typeof p.map !== 'string' && !isMdxRecord(p.map))
    )
      throw new TypeError('Invalid MDX restart source')
    function paths(input: unknown): readonly string[] {
      if (
        !Array.isArray(input) ||
        input.some((path) => typeof path !== 'string')
      )
        throw new TypeError('Invalid MDX restart paths')
      return Object.freeze(
        input.filter((path): path is string => typeof path === 'string'),
      )
    }
    const inputs = value.inputs.map((input: unknown) => {
      if (
        !isMdxRecord(input) ||
        (input.kind !== 'file' &&
          input.kind !== 'build' &&
          input.kind !== 'missing' &&
          input.kind !== 'context') ||
        typeof input.path !== 'string' ||
        typeof input.loader !== 'string' ||
        typeof input.fingerprint !== 'string' ||
        typeof input.mtime !== 'number'
      )
        throw new TypeError('Invalid MDX restart fingerprint')
      return Object.freeze({
        kind: input.kind,
        path: input.path,
        loader: input.loader,
        fingerprint: input.fingerprint,
        mtime: input.mtime,
      })
    })
    const portable = portableValue(value.identity)
    if (
      portable === undefined ||
      (value.sourceMap !== undefined && typeof value.sourceMap !== 'boolean') ||
      (value.mode !== undefined && typeof value.mode !== 'string')
    )
      throw new TypeError('Invalid MDX restart identity')
    const prepared = Object.freeze({
      filename: value.filename,
      source: p.source,
      map: immutableMdxMap(p.map),
      dependencies: paths(p.dependencies),
      contextDependencies: paths(p.contextDependencies),
      missingDependencies: paths(p.missingDependencies),
      buildDependencies: paths(p.buildDependencies),
      dependencyReports: Object.freeze([]),
    })
    if (
      entries.has(value.filename) ||
      !prepared.dependencies.includes(value.filename) ||
      reportedMdxInputs(prepared).some(
        (report) =>
          !inputs.some(
            (input) => input.kind === report.kind && input.path === report.path,
          ),
      ) ||
      inputs.some(
        (input) => input.kind === 'missing' && input.fingerprint !== 'absent',
      )
    )
      throw new TypeError('Incomplete MDX restart input proof')
    entries.set(
      value.filename,
      Object.freeze({
        identity: snapshotMdxBindingValue(portable),
        portable,
        compilerKey: sourceHash(JSON.stringify(portable)),
        sourceMap: value.sourceMap,
        mode: value.mode,
        prepared,
        inputs: Object.freeze(inputs),
      }),
    )
  }
  return entries
}
