import { readdirSync, realpathSync } from 'node:fs'
import { dirname, join } from 'node:path'

import { compareCodePoints } from './import-graph'
import { findPackage } from './owned-module-resolution'
import {
  recordResolutionFailure,
  type ResolutionInputCollector,
  type ResolutionInputObserver,
} from './resolution-inputs'

export function createProductionPackageRoots(
  include: readonly string[],
  excluded: (path: string) => boolean,
  context: {
    readonly inputs: ResolutionInputCollector
    readonly observer: ResolutionInputObserver | undefined
  },
): (start: string) => readonly string[] {
  const discoveries = new Map<string, readonly string[]>()
  const { inputs, observer } = context
  return (start) => {
    const cached = discoveries.get(start)
    if (cached) return cached
    const found = new Set<string>()
    for (let directory = start; ; directory = dirname(directory)) {
      const names = new Set(include)
      for (const scope of ['@devup-ui', '@devup-editor']) {
        const path = join(directory, 'node_modules', scope)
        if (excluded(path)) continue
        try {
          if (excluded(realpathSync.native(path))) continue
          for (const entry of readdirSync(path)) names.add(`${scope}/${entry}`)
        } catch (cause) {
          recordResolutionFailure(path, cause, inputs)
          if (!(
            cause instanceof Error &&
            'code' in cause &&
            (cause.code === 'ENOENT' || cause.code === 'ENOTDIR')
          ))
            throw cause
        } finally {
          observer?.(inputs.snapshot())
        }
      }
      for (const name of names) {
        const path = findPackage(directory, name, excluded, inputs)
        if (path && !excluded(realpathSync.native(path))) found.add(path)
      }
      if (dirname(directory) === directory) break
    }
    observer?.(inputs.snapshot())
    const result = [...found].sort(compareCodePoints)
    discoveries.set(start, result)
    return result
  }
}
