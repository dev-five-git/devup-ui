import { join } from 'node:path'

import type {
  ImportRequestOutcome,
  ResolutionInputs,
} from '@devup-ui/plugin-utils'
import { expect } from 'bun:test'

// Join tests require fixture-owned evidence; exhaustive producer probes live upstream.
export function expectedJoinInputs(
  root: string,
  files: readonly string[],
  missing: readonly string[] = [],
): ResolutionInputs {
  return expect.objectContaining({
    fileDependencies: expect.arrayContaining([...files]),
    missingDependencies: expect.arrayContaining(
      ['tsconfig.json', ...missing].map((path) => join(root, path)),
    ),
  })
}

export function expectedJoinResolution(
  root: string,
  files: readonly [string, ...string[]],
  missing: readonly string[] = [],
): ImportRequestOutcome {
  return {
    kind: 'resolved',
    path: files[0],
    inputs: expectedJoinInputs(root, files, missing),
  }
}
