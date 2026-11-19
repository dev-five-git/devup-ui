import { readFileSync, statSync } from 'node:fs'
import { resolve } from 'node:path'

interface CachedSource {
  readonly stamp: string
  readonly code: string
}

/** The sources one build has read, by absolute path */
export type SourceCache = Map<string, CachedSource>

declare global {
  var __devupSourceCache: SourceCache | undefined
}

/** Changes whenever the file is written; `undefined` once it is gone. */
export function fileStamp(path: string): string | undefined {
  try {
    const { mtimeNs, ctimeNs, size } = statSync(path, { bigint: true })
    return `${mtimeNs}:${ctimeNs}:${size}`
  } catch {
    return undefined
  }
}

/**
 * Run `run` with `cache` holding the sources it reads through `readSource`,
 * so the passes of one build that run before the bundler (the import graph,
 * the class-slot reservation, the prewarm; in Next once per compiler) read
 * each file from disk once. Only a one-shot build passes one cache to several
 * runs: in watch mode each run starts from a fresh one, as without it. A run
 * inside another uses the outer cache. The plugins and their loaders load
 * separate copies of this module, so the active cache lives on `globalThis`.
 */
export function withSourceCache<T>(run: () => T, cache: SourceCache): T {
  const outer = globalThis.__devupSourceCache
  globalThis.__devupSourceCache = outer ?? cache
  try {
    return run()
  } finally {
    globalThis.__devupSourceCache = outer
  }
}

/**
 * The UTF-8 text of `path`. Inside `withSourceCache` it is read from disk
 * again only once the file changes: checking its stamp costs a fraction of
 * reading it.
 */
export function readSource(path: string): string {
  const cache = globalThis.__devupSourceCache
  if (cache === undefined) return readFileSync(path, 'utf-8')
  const key = resolve(path)
  const stamp = fileStamp(key)
  if (stamp === undefined) {
    // Gone: the read reports why, as it would without the cache
    cache.delete(key)
    return readFileSync(key, 'utf-8')
  }
  const cached = cache.get(key)
  if (cached?.stamp === stamp) return cached.code
  const code = readFileSync(key, 'utf-8')
  cache.set(key, { stamp, code })
  return code
}
