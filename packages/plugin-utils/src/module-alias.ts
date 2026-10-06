import type { IgnoredModule } from './import-graph'
import type { ModuleAliases } from './types'

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

export interface AliasResolution {
  readonly ignored?: never
  readonly path: string
  readonly request: string
}

/** Resolve every rewriting candidate completely before selecting its request. */
export function resolveModuleAlias(
  specifier: string,
  context: {
    readonly alias: ModuleAliases
    readonly importer: string
    readonly resolveRequest: (
      request: string,
      aliased: boolean,
    ) => string | false | undefined
  },
): AliasResolution | IgnoredModule | false | undefined {
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
    for (const [key, value] of Object.entries(context.alias)) {
      const exact = key.endsWith('$')
      const name = exact ? key.slice(0, -1) : key
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
      if (targets.length && !candidates.length) continue
      failure ??= {
        key,
        candidates: candidates.filter((candidate) => candidate !== false),
      }
      let missing = !candidates.length
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
