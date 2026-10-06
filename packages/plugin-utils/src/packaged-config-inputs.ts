import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { isAbsolute, join, relative } from 'node:path'

import type { ResolutionInputCollector } from './resolution-inputs'
import { resolutionWatchPath } from './resolution-watch-path'

interface PackagedConfigLookup {
  readonly request: string
  readonly importer: string
  readonly resolved: string | undefined
}

export function recordPackagedConfigInputs(
  lookup: PackagedConfigLookup,
  inputs?: ResolutionInputCollector,
): void {
  if (!inputs) return
  const { request, importer, resolved } = lookup
  const require = createRequire(importer)
  const name = request
    .split('/')
    .slice(0, request.startsWith('@') ? 2 : 1)
    .join('/')
  for (const directory of require.resolve.paths(request) ?? []) {
    const candidate = join(directory, name)
    const exists = existsSync(candidate)
    inputs.probe(candidate, exists)
    if (exists && resolved) {
      const canonical = resolutionWatchPath(candidate)
      const subpath = relative(canonical, resolved)
      if (!subpath.startsWith('..') && !isAbsolute(subpath)) break
    }
  }
}
