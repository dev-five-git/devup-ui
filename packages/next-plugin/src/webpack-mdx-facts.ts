import { pathToFileURL } from 'node:url'

import { isMdxRecord, type MdxLoader } from './mdx-pipeline'
import { resolveWebpackLoader } from './webpack-mdx-resolution'
import { WebpackResourceError } from './webpack-resource-error'
import type { WebpackResourceBinding } from './webpack-resource-native'
import { isDevupLoader } from './webpack-resource-qualification'

export function createWebpackLoaderFacts(
  binding: WebpackResourceBinding,
  configFile: string,
) {
  const watches = new Set<string>()
  const loaded = new Map<
    string,
    Awaited<ReturnType<typeof resolveWebpackLoader>>
  >()
  async function resolveLoader(loader: MdxLoader, signal: AbortSignal) {
    signal.throwIfAborted()
    let result = loaded.get(loader.loader)
    if (!result) {
      result = await resolveWebpackLoader(binding, loader, signal)
      signal.throwIfAborted()
      loaded.set(loader.loader, result)
    }
    signal.throwIfAborted()
    result.dependencies.forEach((dependency) => watches.add(dependency))
    return Object.freeze({
      ...result,
      loader: Object.freeze({ ...loader, loader: result.facts.resolvedPath }),
      facts: Object.freeze({ ...result.facts, options: loader.options }),
    })
  }
  return Object.freeze({
    resolveLoader,
    watchInputs: () => Object.freeze([...watches]),
    async downstreamSafe(
      loaders: readonly MdxLoader[],
      filename: string,
      signal: AbortSignal,
    ) {
      for (const loader of loaders) {
        if (isDevupLoader(loader)) continue
        const result = await resolveLoader(loader, signal)
        const module: unknown = await import(
          pathToFileURL(result.facts.resolvedPath).href
        )
        const entry: unknown = isMdxRecord(module) ? module.default : module
        const pitch =
          typeof entry === 'function' && 'pitch' in entry
            ? entry.pitch
            : isMdxRecord(module)
              ? module.pitch
              : undefined
        if (pitch === undefined) continue
        const swc =
          result.facts.resolvedPath
            .replaceAll('\\', '/')
            .endsWith('/next/dist/build/webpack/loaders/next-swc-loader.js') &&
          result.facts.packageVersion === '16.3.6' &&
          result.facts.verifiedFileHash ===
            '6c1e45b146eae04f982b91e06170ab349e00686ebe021da48262218cbf95f604'
        if (!swc)
          throw new WebpackResourceError(configFile, {
            filename,
            rulePosition: 'module.rules',
            test: '<native downstream>',
            unknownFact: 'pitch/source substitution',
            reason: `downstream loader ${result.facts.resolvedPath} has an unproved pitch`,
          })
      }
    },
  })
}
