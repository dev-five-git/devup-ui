import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'

import {
  nextNonphysicalIdList,
  type NonphysicalIdListOutcome,
  parseNonphysicalIdList,
  serializeNonphysicalIdList,
} from './nonphysical-id-list'
import { resolveProjectPaths } from './shared'
import { createStateWriter } from './state-writer'

export interface NonphysicalIdStoreScope {
  readonly integration: string
  readonly resolvedRoot: string
  readonly contextKey: string
  readonly distDir?: string
}

export interface NonphysicalIdStore {
  readonly filePath: string
  read(): Promise<NonphysicalIdListOutcome>
  rewrite(
    extractedIds: readonly string[],
    scanReservedIds: readonly string[],
  ): Promise<void>
}

export function createNonphysicalIdStore(
  scope: NonphysicalIdStoreScope,
): NonphysicalIdStore {
  const { distDir } = resolveProjectPaths(
    scope.resolvedRoot,
    scope.distDir === undefined ? {} : { distDir: scope.distDir },
  )
  const key = createHash('sha256')
    .update(
      JSON.stringify([scope.integration, scope.resolvedRoot, scope.contextKey]),
    )
    .digest('hex')
  const filePath = join(distDir, 'numbering', `${key}.json`)
  const writer = createStateWriter()
  return {
    filePath,
    async read() {
      let serialized: string
      try {
        serialized = await readFile(filePath, 'utf-8')
      } catch (error) {
        if (
          error instanceof Error &&
          'code' in error &&
          error.code === 'ENOENT'
        )
          return parseNonphysicalIdList(undefined)
        throw error
      }
      return parseNonphysicalIdList(serialized)
    },
    rewrite(extractedIds, scanReservedIds) {
      return writer.write(
        filePath,
        serializeNonphysicalIdList(
          nextNonphysicalIdList(extractedIds, scanReservedIds),
        ),
      )
    },
  }
}
