import { readFileSync } from 'node:fs'
import { basename, dirname, resolve } from 'node:path'

/** Per-preparation classification: missing/invalid scopes must remain candidates. */
export function packageBoundary() {
  const cache = new Map<string, boolean>()
  return (directory: string): boolean => {
    const cached = cache.get(directory)
    if (cached !== undefined) return cached
    const parent = dirname(directory)
    const packageRoot =
      basename(parent) === 'node_modules'
        ? !basename(directory).startsWith('@')
        : basename(parent).startsWith('@') &&
          basename(dirname(parent)) === 'node_modules'
    let boundary = false
    if (
      !directory.replaceAll('\\', '/').includes('/node_modules/') ||
      packageRoot
    ) {
      try {
        const manifest: unknown = JSON.parse(
          readFileSync(resolve(directory, 'package.json'), 'utf8').replace(
            /^\uFEFF/,
            '',
          ),
        )
        boundary =
          typeof manifest === 'object' &&
          manifest !== null &&
          'name' in manifest &&
          typeof manifest.name === 'string' &&
          manifest.name.length > 0 &&
          (!('sideEffects' in manifest) ||
            !Array.isArray(manifest.sideEffects) ||
            manifest.sideEffects.every(
              (value: unknown) => typeof value === 'string',
            ))
      } catch (error) {
        if (!(error instanceof Error)) throw error
        // Vite skips unreadable or unparseable package scopes as well.
        boundary = false
      }
    }
    cache.set(directory, boundary)
    return boundary
  }
}
