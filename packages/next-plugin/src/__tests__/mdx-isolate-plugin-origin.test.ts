import { execFile } from 'node:child_process'
import { join } from 'node:path'
import { promisify } from 'node:util'

import { expect, it } from 'bun:test'

import { isMdxRecord } from '../mdx-pipeline'

const execute = promisify(execFile)

it.each([
  { runtime: 'node', format: 'cjs', mode: 'construct' },
  { runtime: 'node', format: 'cjs', mode: 'nested' },
  { runtime: process.execPath, format: 'esm', mode: 'construct' },
  { runtime: process.execPath, format: 'esm', mode: 'nested' },
])(
  'quarantines the installed plugin diagnostic before B: %j',
  async ({ runtime, format, mode }) => {
    // Given
    const driver = join(
      import.meta.dir,
      'mdx-isolate-plugin-origin-runtime.mjs',
    )
    // When
    let childFailure: unknown
    const result = await execute(runtime, [driver, format, mode], {
      timeout: 25_000,
      maxBuffer: 1024 * 1024,
    }).catch((cause: unknown) => {
      childFailure = cause
      if (
        isMdxRecord(cause) &&
        typeof cause.stdout === 'string' &&
        typeof cause.stderr === 'string'
      )
        return { stdout: cause.stdout, stderr: cause.stderr }
      throw cause
    })
    // Then
    const observation: unknown = JSON.parse(result.stdout)
    expect(observation).toMatchObject({ workerBExecutions: 0, publications: 0 })
    expect(childFailure).toBeUndefined()
    expect(result.stderr).toBe('')
  },
  30_000,
)
