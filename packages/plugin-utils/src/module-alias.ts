export class ModuleAliasError extends Error {
  constructor(importer: string, specifier: string) {
    super(`${importer}:1:1: Module alias cycle cannot resolve ${specifier}`)
    this.name = 'ModuleAliasError'
  }
}

/** First rewriting match wins; self aliases are not rewriting matches. */
export function rewriteModuleAlias(
  specifier: string,
  alias: Readonly<Record<string, string>>,
  importer: string,
): string {
  const visited = new Set<string>()
  let request = specifier
  while (!visited.has(request)) {
    visited.add(request)
    let rewritten = request
    for (const [key, target] of Object.entries(alias)) {
      const exact = key.endsWith('$')
      const name = exact ? key.slice(0, -1) : key
      if (request !== name && (exact || !request.startsWith(`${name}/`)))
        continue
      if (request === target || request.startsWith(`${target}/`)) continue
      const candidate = target + request.slice(name.length)
      rewritten = candidate
      break
    }
    if (rewritten === request) return request
    request = rewritten
  }
  throw new ModuleAliasError(importer, specifier)
}
