import { resolutionWatchPath } from '@devup-ui/plugin-utils'
import type { Compiler, LoaderContext } from 'webpack'

import { MdxFreshnessError } from './mdx-source-freshness'

export function loaderResolutionWatchPath(
  path: string,
  compiler: Compiler | undefined,
): string {
  const symlinks = compiler?.options.resolve?.symlinks
  return typeof symlinks === 'boolean'
    ? resolutionWatchPath(path, symlinks === false)
    : path
}

export async function registerLoaderMissingDependencies(
  context: Pick<
    LoaderContext<unknown>,
    '_compiler' | 'addMissingDependency' | 'fs' | 'resourcePath'
  >,
  paths: readonly string[],
): Promise<void> {
  for (const path of paths) {
    const missing = loaderResolutionWatchPath(path, context._compiler)
    context.addMissingDependency(missing)
    if (typeof context.fs?.readFile !== 'function')
      throw new TypeError(
        `Exact missing input tracking requires loader fs.readFile for ${missing}`,
      )
    await new Promise<void>((resolve, reject) => {
      context.fs.readFile(missing, (error) => {
        if (error?.code === 'ENOENT' || error?.code === 'ENOTDIR') resolve()
        else if (error) reject(error)
        else
          reject(
            new MdxFreshnessError(
              context.resourcePath,
              {
                kind: 'missing',
                path: missing,
                loader: '@devup-ui/next-plugin/loader',
              },
              'observed missing resolution input became readable during delivery',
            ),
          )
      })
    })
  }
}
