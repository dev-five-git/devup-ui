import { isMdxRecord } from './mdx-pipeline'

export type PreparedMdx = {
  readonly filename: string
  readonly source: string
  readonly map: string | Readonly<Record<string, unknown>> | undefined
  readonly dependencies: readonly string[]
  readonly contextDependencies: readonly string[]
  readonly missingDependencies: readonly string[]
}

function fileDependencies(value: unknown): string[] {
  if (!Array.isArray(value) || value.some((file) => typeof file !== 'string'))
    throw new TypeError('invalid loader dependency list')
  return value.filter((file): file is string => typeof file === 'string')
}

export function prepared(filename: string, value: unknown): PreparedMdx {
  if (!isMdxRecord(value) || !Array.isArray(value.result))
    throw new TypeError('loader runner returned no compiled source')
  const [source, map] = value.result
  if (typeof source !== 'string' && !Buffer.isBuffer(source))
    throw new TypeError('MDX compiler returned no JavaScript source')
  if (map != null && typeof map !== 'string' && !isMdxRecord(map))
    throw new TypeError('MDX compiler returned an invalid source map')
  return {
    filename,
    source: source.toString(),
    map: map ?? undefined,
    dependencies: [
      ...new Set([filename, ...fileDependencies(value.fileDependencies)]),
    ],
    contextDependencies: fileDependencies(value.contextDependencies),
    missingDependencies: fileDependencies(value.missingDependencies),
  }
}
