import type { IgnoredModule } from './import-graph'
import type { ModuleAliasDescriptor, ModuleAliasOptions } from './types'

export class ModuleAliasError extends Error {
  constructor(importer: string, specifier: string) {
    super(`${importer}:1:1: Module alias cycle cannot resolve ${specifier}`)
    this.name = 'ModuleAliasError'
  }
}

export class ModuleAliasCandidatesError extends Error {
  readonly name = 'ModuleAliasCandidatesError'
  constructor(
    readonly importer: string,
    readonly key: string,
    readonly candidates: readonly string[],
  ) {
    super(
      `${importer}:1:1: Module alias ${key} cannot resolve candidates ${JSON.stringify(candidates)}`,
    )
  }
}

export class ModuleAliasPackageError extends Error {
  readonly name = 'ModuleAliasPackageError'
  constructor(
    readonly importer: string,
    readonly request: string,
  ) {
    super(
      `${importer}:1:1: Module alias package ${request} has no resolving export under the active conditions`,
    )
  }
}

export class ModuleAliasConfigurationError extends Error {
  readonly name = 'ModuleAliasConfigurationError'
  constructor(
    readonly importer: string,
    readonly key: string,
  ) {
    super(
      `${importer}:1:1: Module alias ${key} cannot use wildcard names in the shared resolver`,
    )
  }
}

export interface AliasResolution {
  readonly ignored?: never
  readonly path: string
  readonly request: string
}

/** Resolve every rewriting candidate completely before selecting its request. */
export function resolveModuleAlias(
  specifier: string,
  context: {
    readonly alias: ModuleAliasOptions
    readonly importer: string
    readonly resolveRequest: (
      request: string,
      aliased: boolean,
    ) => string | false | undefined
  },
): AliasResolution | IgnoredModule | false | undefined {
  const descriptors: readonly (ModuleAliasDescriptor & {
    readonly key: string
  })[] =
    context.alias === false
      ? []
      : Array.isArray(context.alias)
        ? context.alias.map((entry) => ({ ...entry, key: entry.name }))
        : Object.entries(context.alias).map(([key, alias]) => ({
            key,
            name: key.endsWith('$') ? key.slice(0, -1) : key,
            onlyModule: key.endsWith('$'),
            alias,
          }))
  let failure:
    { readonly key: string; readonly candidates: readonly string[] } | undefined
  function visit(
    request: string,
    visited: ReadonlySet<string>,
    aliased: boolean,
  ): AliasResolution | IgnoredModule | false | undefined {
    if (visited.has(request))
      throw new ModuleAliasError(context.importer, specifier)
    const next = new Set(visited).add(request)
    for (const { key, name, alias: value, onlyModule: exact } of descriptors) {
      if (name.includes('*'))
        throw new ModuleAliasConfigurationError(context.importer, key)
      if (request !== name && (exact || !request.startsWith(`${name}/`)))
        continue
      const targets = typeof value === 'object' ? value : [value]
      const candidates = targets
        .filter(
          (target) =>
            target === false ||
            (request !== target && !request.startsWith(`${target}/`)),
        )
        .map((target) =>
          target === false ? false : target + request.slice(name.length),
        )
      if (!candidates.length) continue
      failure ??= {
        key,
        candidates: candidates.filter((candidate) => candidate !== false),
      }
      let missing = false
      for (const candidate of candidates) {
        if (candidate === false) return { ignored: true }
        const resolved = visit(candidate, next, true)
        if (resolved) return resolved
        if (resolved === undefined) missing = true
      }
      return missing ? undefined : false
    }
    const path = context.resolveRequest(request, aliased)
    return typeof path === 'string' ? { path, request } : path
  }
  const resolved = visit(specifier, new Set(), false)
  if (resolved === undefined && failure)
    throw new ModuleAliasCandidatesError(
      context.importer,
      failure.key,
      failure.candidates,
    )
  return resolved
}
