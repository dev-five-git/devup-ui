import {
  type CompiledReference,
  createCompileTimeClassifier,
  isCompileTimeAlias,
  UntransformedSourceError,
} from './compiled-guard'
import { isSelectedSource } from './source-selection'
import type { WasmImportAliases } from './types'

interface UsedDependency {
  readonly request: string
  readonly type: string
  readonly ids?: unknown
  readonly names?: unknown
  readonly getIds?: (graph: unknown) => unknown
  readonly loc?: unknown
}

function isDependency(value: unknown): value is UsedDependency {
  return (
    !!value &&
    typeof value === 'object' &&
    'request' in value &&
    typeof value.request === 'string' &&
    'type' in value &&
    typeof value.type === 'string'
  )
}

function stringIds(value: unknown): readonly string[] {
  return Array.isArray(value) &&
    value.every((id: unknown) => typeof id === 'string')
    ? value
    : []
}

export interface GuardModule {
  readonly resource?: string
  readonly type: string
  readonly dependencies: readonly unknown[]
}

export interface DependencyGuardOptions {
  readonly package: string
  readonly mdxExtensions: readonly string[]
  readonly importAliases: WasmImportAliases
}

export function createDependencyGuard(options: DependencyGuardOptions) {
  const classify = createCompileTimeClassifier(options.package)
  return (
    modules: Iterable<GuardModule>,
    context: {
      readonly graph: unknown
      readonly target: (dependency: unknown) => string | undefined
      readonly forwarded?: (
        dependency: unknown,
        ids: readonly string[],
      ) => boolean
    },
  ): UntransformedSourceError[] => {
    const errors: UntransformedSourceError[] = []
    for (const module of modules) {
      if (
        !module.resource ||
        !module.type.startsWith('javascript') ||
        isSelectedSource(module.resource.split('?')[0], options.mdxExtensions)
      )
        continue
      for (const dependency of module.dependencies) {
        if (
          !isDependency(dependency) ||
          ![
            'harmony import specifier',
            'esm import specifier',
            'cjs full require',
            'cjs require',
          ].includes(dependency.type)
        )
          continue
        const ids = stringIds(
          typeof dependency.getIds === 'function'
            ? dependency.getIds(context.graph)
            : (dependency.ids ?? dependency.names),
        )
        if (!ids.length) continue
        const loc = dependency.loc
        const start =
          loc && typeof loc === 'object' && 'start' in loc
            ? loc.start
            : undefined
        const reference: CompiledReference = {
          request: dependency.request,
          ids,
          line:
            start &&
            typeof start === 'object' &&
            'line' in start &&
            typeof start.line === 'number'
              ? start.line
              : 1,
          column:
            start &&
            typeof start === 'object' &&
            'column' in start &&
            typeof start.column === 'number'
              ? start.column + 1
              : 1,
        }
        const target = context.target(dependency)
        if (
          isCompileTimeAlias(reference, options.importAliases) ||
          (target && classify(target, ids)) ||
          context.forwarded?.(dependency, ids)
        )
          errors.push(
            new UntransformedSourceError(
              module.resource,
              reference,
              isCompileTimeAlias(reference, options.importAliases),
            ),
          )
      }
    }
    return errors
  }
}
