import { realpathSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'

function canonicalWatchPath(path: string, parent: string): string {
  try {
    return join(realpathSync(parent), relative(parent, path))
  } catch (error) {
    if (
      !(error instanceof Error) ||
      !('code' in error) ||
      (error.code !== 'ENOENT' && error.code !== 'ENOTDIR')
    )
      return path
    const next = dirname(parent)
    return next === parent ? path : canonicalWatchPath(path, next)
  }
}

/** A caller-owned memo for one observer lifetime: each resolution re-reports the same inputs thousands of times. */
export function resolutionWatchPath(
  path: string,
  preserveSymlinks = false,
  cache?: Map<string, string>,
): string {
  if (preserveSymlinks) return path
  const known = cache?.get(path)
  if (known !== undefined) return known
  const canonical = canonicalWatchPath(path, path)
  cache?.set(path, canonical)
  return canonical
}
