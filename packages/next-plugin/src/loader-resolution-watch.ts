import { resolutionWatchPath } from '@devup-ui/plugin-utils'
import type { Compiler } from 'webpack'

export function loaderResolutionWatchPath(
  path: string,
  compiler: Compiler | undefined,
): string {
  const symlinks = compiler?.options.resolve?.symlinks
  return typeof symlinks === 'boolean'
    ? resolutionWatchPath(path, symlinks === false)
    : path
}
