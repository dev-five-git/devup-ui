import { posix, win32 } from 'node:path'

export interface NonphysicalModuleId {
  readonly namespace: string | null
  readonly id: string
}

export interface NonphysicalRequest {
  readonly namespace: string | null
  readonly specifier: string
}

export interface PhysicalImporter {
  readonly path: string
  readonly pathStyle: 'posix' | 'win32'
}

type Exhaustive<Remaining extends never> = Remaining

export function normalizeNonphysicalModuleId(
  resolved: NonphysicalModuleId | undefined,
  request: NonphysicalRequest,
  importer: PhysicalImporter | undefined,
): NonphysicalModuleId {
  if (resolved !== undefined)
    return Object.freeze({ namespace: resolved.namespace, id: resolved.id })
  const specifier = request.specifier
  if (importer === undefined || specifier.includes('\0'))
    return Object.freeze({ namespace: request.namespace, id: specifier })
  switch (importer.pathStyle) {
    case 'posix':
      if (!/^\.\.?\//.test(specifier))
        return Object.freeze({ namespace: request.namespace, id: specifier })
      return relativeIdentity(request, importer.path, posix)
    case 'win32':
      if (!/^\.\.?[/\\]/.test(specifier))
        return Object.freeze({ namespace: request.namespace, id: specifier })
      return relativeIdentity(request, importer.path, win32)
  }
  type _RemainingPathStyle = Exhaustive<typeof importer.pathStyle>
}

function relativeIdentity(
  request: NonphysicalRequest,
  importerPath: string,
  paths: Pick<typeof posix, 'dirname' | 'resolve'>,
): NonphysicalModuleId {
  const specifier = request.specifier
  const boundary = specifier.search(/[?#]/)
  const path = boundary === -1 ? specifier : specifier.slice(0, boundary)
  const suffix = boundary === -1 ? '' : specifier.slice(boundary)
  return Object.freeze({
    namespace: request.namespace,
    id: paths.resolve(paths.dirname(importerPath), path) + suffix,
  })
}
