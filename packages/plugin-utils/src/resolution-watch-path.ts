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

export function resolutionWatchPath(
  path: string,
  preserveSymlinks = false,
): string {
  return preserveSymlinks ? path : canonicalWatchPath(path, path)
}
