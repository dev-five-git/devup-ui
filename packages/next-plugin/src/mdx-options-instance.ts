import { isDeepStrictEqual } from 'node:util'

import { isMdxRecord, type MdxLoader, type MdxPipeline } from './mdx-pipeline'

export type MdxOptionsInstance = {
  readonly loadersFor: (pipeline: MdxPipeline) => readonly MdxLoader[]
}

export class MdxTurboOptionsError extends TypeError {
  readonly name = 'MdxTurboOptionsError'
  constructor(
    readonly loader: string,
    readonly ruleKey: string,
    readonly detail: string,
  ) {
    super(`loader ${loader} for match "${ruleKey}" ${detail}`)
  }
}

function webpackCompiler(loader: MdxLoader): MdxLoader {
  if (!isMdxRecord(loader.options)) return loader
  // Only the approved wrapper-mutated shells are detached. Plugin options stay shared.
  const options = { ...loader.options }
  for (const key of ['remarkPlugins', 'rehypePlugins', 'recmaPlugins']) {
    const plugins: unknown = options[key]
    if (Array.isArray(plugins))
      options[key] = plugins.map((plugin: unknown) =>
        Array.isArray(plugin) ? [...plugin] : plugin,
      )
  }
  return { ...loader, options }
}

function turboLoader(loader: MdxLoader, ruleKey: string): MdxLoader {
  // Next 16.3.6 swc/index.js checkLoaderItem: non-JSON values are rejected,
  // not normalized. Native WebpackLoaderItem has a JSON map and defaults to {}.
  const serialized: unknown = JSON.parse(JSON.stringify(loader))
  if (!isMdxRecord(serialized) || !isDeepStrictEqual(loader, serialized))
    throw new MdxTurboOptionsError(
      loader.loader,
      ruleKey,
      'does not have serializable options. Ensure that options passed are plain JavaScript objects and values.',
    )
  const options = serialized.options === undefined ? {} : serialized.options
  if (!isMdxRecord(options))
    throw new MdxTurboOptionsError(
      loader.loader,
      ruleKey,
      'requires an object options map on the native side',
    )
  // Native JSON -> Node pool JSON.parse -> transform forwards this object to getOptions.
  return { loader: loader.loader, options }
}

// Create once per preparation run, reuse for every file, discard before dev re-preparation.
export function createMdxOptionsInstance(): MdxOptionsInstance {
  const instances = new WeakMap<MdxPipeline, readonly MdxLoader[]>()
  return {
    loadersFor(pipeline) {
      const existing = instances.get(pipeline)
      if (existing) return existing
      let loaders: readonly MdxLoader[]
      switch (pipeline.bundler) {
        case 'webpack':
          loaders = pipeline.loaders.map((loader, index) =>
            index === 0 ? webpackCompiler(loader) : loader,
          )
          break
        case 'turbo':
          loaders = pipeline.loaders.map((loader) =>
            turboLoader(loader, pipeline.ruleKey),
          )
          break
        default:
          return pipeline.bundler satisfies never
      }
      instances.set(pipeline, loaders)
      return loaders
    },
  }
}
