import { statSync } from 'node:fs'
import { dirname } from 'node:path'

import { resolutionWatchPath } from '@devup-ui/plugin-utils'
import type { Compiler, LoaderContext } from 'webpack'

export function loaderResolutionWatchPath(
  path: string,
  compiler: Compiler | undefined,
): string {
  const symlinks = compiler?.options.resolve?.symlinks
  return typeof symlinks === 'boolean'
    ? resolutionWatchPath(path, symlinks === false)
    : path
}

export function registerLoaderMissingDependencies(
  context: Pick<
    LoaderContext<unknown>,
    '_compiler' | 'addMissingDependency' | 'addContextDependency'
  >,
  paths: readonly string[],
): void {
  const ancestors = new Set<string>()
  for (const path of paths) {
    const missing = loaderResolutionWatchPath(path, context._compiler)
    context.addMissingDependency(missing)
    let ancestor = dirname(missing)
    while (true) {
      try {
        if (statSync(ancestor, { throwIfNoEntry: false })?.isDirectory()) break
      } catch (error) {
        if (
          !(error instanceof Error) ||
          !('code' in error) ||
          (error.code !== 'ENOENT' && error.code !== 'ENOTDIR')
        )
          throw error
      }
      ancestor = dirname(ancestor)
    }
    ancestors.add(ancestor)
  }
  // Turbopack transports contexts, but not loader-runner's missing category.
  for (const ancestor of ancestors) context.addContextDependency(ancestor)
}
