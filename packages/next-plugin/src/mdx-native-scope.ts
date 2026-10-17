import { createHash } from 'node:crypto'

import {
  compareMdxBindingValues,
  snapshotMdxBindingValue,
} from './mdx-binding-value'
import { isMdxRecord } from './mdx-pipeline'

type Snapshot = ReturnType<typeof snapshotMdxBindingValue>
type Scope = { readonly token: object; readonly cells: Map<string, Snapshot> }
const modules = new WeakMap<
  object,
  { readonly compilers: WeakMap<object, Scope>; marker?: Scope }
>()

function nativeOptionsHash(options: Readonly<Record<string, unknown>>) {
  const hash = createHash('sha256')
  for (const key in options) {
    if (!Object.hasOwn(options, key)) continue
    const value: unknown = options[key]
    if (value !== undefined) hash.update(key + JSON.stringify(value))
  }
  return hash.digest('hex').slice(0, 16)
}

export class MdxNativeCellError extends Error {
  readonly name = 'MdxNativeCellError'
  constructor(
    readonly option: string,
    readonly detail: string,
  ) {
    super(
      `native MDX cache cell option ${option}: ${detail}; use distinct native Compiler instances or distinguishable serializable compiler options`,
    )
  }
}

export function requireMdxSafeCompilerOptions(
  value: unknown,
  phase: 'wire' | 'hash' = 'hash',
) {
  const seen = new Set<object>()
  function visit(input: unknown, path: string) {
    if (
      (typeof input !== 'object' || input === null) &&
      typeof input !== 'function'
    )
      return
    if (seen.has(input)) {
      if (phase === 'wire') return
      throw new MdxNativeCellError(
        path,
        'cyclic compiler options cannot be hashed safely',
      )
    }
    if ('toJSON' in input) {
      if (
        input instanceof Date &&
        Object.getPrototypeOf(input) === Date.prototype &&
        [
          'toJSON',
          'valueOf',
          'toString',
          'toISOString',
          Symbol.toPrimitive,
        ].every((key) => !Object.hasOwn(input, key))
      )
        return
      throw new MdxNativeCellError(
        path,
        'effectful toJSON cannot be evaluated speculatively',
      )
    }
    if (typeof input === 'function') return
    seen.add(input)
    for (const key of Reflect.ownKeys(input)) {
      const descriptor = Object.getOwnPropertyDescriptor(input, key)
      if (!descriptor || !('value' in descriptor))
        throw new MdxNativeCellError(
          `${path}.${String(key)}`,
          'compiler option accessor cannot be evaluated speculatively',
        )
      const child: unknown = descriptor.value
      visit(child, `${path}.${String(key)}`)
    }
    seen.delete(input)
  }
  visit(value, 'options')
}

export function admitMdxNativeCell(
  module: object,
  compiler: object | null | undefined,
  effective: {
    readonly options: unknown
    readonly sourceMap: unknown
    readonly mode: unknown
  },
) {
  if (compiler === undefined)
    throw new MdxNativeCellError(
      '_compiler',
      'native Compiler identity is unavailable; obtain the actual native context before preparation',
    )
  requireMdxSafeCompilerOptions(effective.options)
  if (!isMdxRecord(effective.options))
    throw new MdxNativeCellError('options', 'MDX requires an options object')
  let scopes = modules.get(module)
  if (!scopes) modules.set(module, (scopes = { compilers: new WeakMap() }))
  let scope = compiler === null ? scopes.marker : scopes.compilers.get(compiler)
  if (!scope) {
    scope = { token: Object.freeze(Object.create(null)), cells: new Map() }
    if (compiler === null) scopes.marker = scope
    else scopes.compilers.set(compiler, scope)
  }
  const options = effective.options
  const key = nativeOptionsHash(options)
  const snapshot = snapshotMdxBindingValue({
    options: effective.options,
    sourceMap: Object.hasOwn(effective.options, 'SourceMapGenerator')
      ? effective.options.SourceMapGenerator
      : Boolean(effective.sourceMap),
    development: Object.hasOwn(effective.options, 'development')
      ? effective.options.development
      : effective.mode === 'development',
  })
  const previous = scope.cells.get(key)
  const difference = previous && compareMdxBindingValues(previous, snapshot)
  if (difference)
    throw new MdxNativeCellError(
      difference.path,
      `incompatible configuration has the same native ordered options hash (${difference.reason})`,
    )
  if (!previous) scope.cells.set(key, snapshot)
  return Object.freeze({
    compiler: scope.token,
    current() {
      try {
        requireMdxSafeCompilerOptions(options)
        return nativeOptionsHash(options) === key
      } catch (cause) {
        if (cause instanceof MdxNativeCellError) return false
        throw cause
      }
    },
  })
}
