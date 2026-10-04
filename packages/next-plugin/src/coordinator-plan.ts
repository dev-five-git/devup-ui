import { getFileNumByFilename } from '@devup-ui/plugin-utils'

import { createCompletionTracker } from './coordinator-completion'
import { locatedError } from './coordinator-engine'
import { HttpError } from './coordinator-http'
import type { CoordinatorOptions } from './coordinator-options'

export interface ProductionPlan {
  close(): void
  succeed(file: string): void
  fail(file: string, message: string): void
  forget(file: string): void
  /** Record which bucket the CSS file an extraction produced belongs to. */
  note(filename: string, cssFile: string | undefined): void
  /** Resolves once everything the stylesheet for `fileNum` needs was extracted. */
  wait(fileNum: number | undefined): Promise<void>
}

type PlanOptions = Pick<
  CoordinatorOptions,
  'canonicalMap' | 'expectedBaseFiles' | 'maxWaitMs' | 'prewarmedFiles'
>

/** Canonical bucket -> every file whose CSS the bucket's chunk holds. */
function bucketMembers(
  canonicalMap: Record<string, string>,
): Map<string, Set<string>> {
  const members = new Map<string, Set<string>>()
  for (const [member, bucket] of Object.entries(canonicalMap)) {
    // `@global` files contribute to the base sheet, not a numbered bucket.
    if (bucket === '@global') continue
    // The bucket root is itself a member of its own chunk.
    const set = members.get(bucket) ?? new Set([bucket])
    members.set(bucket, set.add(member))
  }
  return members
}

/**
 * What a production stylesheet is waiting for. The base sheet needs every
 * expected file; a bucket needs all of its members. A plan with neither
 * `prewarmedFiles` nor `expectedBaseFiles` cannot say when the base sheet is
 * complete, so asking for it is an error rather than a guess.
 */
export function createProductionPlan(options: PlanOptions): ProductionPlan {
  const expectedBase = options.expectedBaseFiles ?? []
  const planned =
    options.prewarmedFiles !== undefined || expectedBase.length > 0
  const buckets = bucketMembers(options.canonicalMap)
  const fileNumToBucket = new Map<number, string>()
  const completion = createCompletionTracker(options.maxWaitMs ?? 60_000)
  for (const file of options.prewarmedFiles ?? []) completion.succeed(file)
  return {
    close: () => completion.close(),
    succeed: (file) => completion.succeed(file),
    fail: (file, message) => completion.fail(file, message),
    forget: (file) => completion.forget(file),
    note(filename, cssFile) {
      const fileNum = cssFile ? getFileNumByFilename(cssFile) : null
      if (fileNum !== null) {
        fileNumToBucket.set(fileNum, options.canonicalMap[filename] ?? filename)
      }
    },
    wait(fileNum) {
      const bucket =
        fileNum === undefined ? undefined : fileNumToBucket.get(fileNum)
      if (bucket !== undefined) {
        return completion.wait(`bucket ${bucket}`, [
          ...(buckets.get(bucket) ?? [bucket]),
        ])
      }
      if (!planned) {
        throw new HttpError(
          500,
          locatedError(
            'devup-ui.css',
            'serve a complete production stylesheet',
            'neither prewarmedFiles nor expectedBaseFiles says which files it needs',
            'prewarm the planned files and pass prewarmedFiles (an empty list is an empty stylesheet), or pass expectedBaseFiles.',
          ).message,
        )
      }
      return completion.wait('the base stylesheet', expectedBase)
    },
  }
}
