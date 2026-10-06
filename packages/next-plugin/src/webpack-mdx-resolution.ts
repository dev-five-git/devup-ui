import { createHash } from 'node:crypto'

import type { MdxNativeLoaderFacts } from './mdx-binding'
import { isMdxRecord, type MdxLoader } from './mdx-pipeline'
import { readWebpackLoaderBytes } from './webpack-mdx-bytes'
import type { WebpackResourceBinding } from './webpack-resource-native'

type Resolver = {
  readonly resolve: (
    info: Readonly<Record<string, unknown>>,
    context: string,
    request: string,
    dependencies: {
      readonly fileDependencies: Set<string>
      readonly missingDependencies: Set<string>
      readonly contextDependencies: Set<string>
    },
    callback: (error: unknown, file: unknown, data: unknown) => void,
  ) => void
}
function isResolver(value: unknown): value is Resolver {
  return isMdxRecord(value) && typeof value.resolve === 'function'
}

export async function resolveWebpackLoader(
  binding: WebpackResourceBinding,
  loader: MdxLoader,
  signal: AbortSignal,
): Promise<{
  readonly loader: MdxLoader
  readonly facts: MdxNativeLoaderFacts
  readonly dependencies: readonly string[]
}> {
  signal.throwIfAborted()
  const resolver: unknown = binding.compiler.resolverFactory.get('loader')
  if (!isResolver(resolver))
    throw new TypeError('native loader resolver unavailable')
  const dependencies = {
    fileDependencies: new Set<string>(),
    missingDependencies: new Set<string>(),
    contextDependencies: new Set<string>(),
  }
  const resolved = await new Promise<{
    readonly file: string
    readonly version: string
  }>((resolve, reject) =>
    resolver.resolve(
      { compiler: binding.compiler.name },
      binding.compiler.context,
      loader.loader,
      dependencies,
      (error, file, data) => {
        if (error) return reject(error)
        if (
          typeof file !== 'string' ||
          !isMdxRecord(data) ||
          !isMdxRecord(data.descriptionFileData) ||
          typeof data.descriptionFileData.version !== 'string'
        )
          return reject(
            new TypeError(
              'native loader resolution did not provide installed version facts',
            ),
          )
        resolve({ file, version: data.descriptionFileData.version })
      },
    ),
  )
  signal.throwIfAborted()
  const bytes = await readWebpackLoaderBytes(resolved.file, signal)
  const facts = Object.freeze({
    resolvedPath: resolved.file,
    packageVersion: resolved.version,
    verifiedFileHash: createHash('sha256').update(bytes).digest('hex'),
    options: loader.options,
  })
  return Object.freeze({
    loader: Object.freeze({ ...loader, loader: resolved.file }),
    facts,
    dependencies: Object.freeze([
      ...dependencies.fileDependencies,
      ...dependencies.missingDependencies,
      ...dependencies.contextDependencies,
      resolved.file,
    ]),
  })
}
