import { execFile } from 'node:child_process'
import { join } from 'node:path'
import { promisify } from 'node:util'

import { expect, it } from 'bun:test'

const execute = promisify(execFile)

it('refuses changed worker-local current H before resource binding', async () => {
  // Given
  const driver = join(import.meta.dir, 'mdx-isolate-runtime.mjs')
  // When
  const execution = execute('node', [driver, 'cjs', 'current-h'], {
    timeout: 25_000,
    maxBuffer: 1024 * 1024,
  })
  // Then
  await expect(execution).resolves.toMatchObject({ stderr: '' })
}, 30_000)

it.each(['node', process.execPath])(
  'proves package-built dormant isolation and cleanup on %s',
  async (runtime) => {
    // Given
    const driver = join(import.meta.dir, 'mdx-isolate-runtime.mjs')
    // When
    const { stdout, stderr } = await execute(runtime, [driver, 'cjs'], {
      timeout: 25_000,
      maxBuffer: 1024 * 1024,
    })
    // Then
    expect(stderr).toBe('')
    expect(JSON.parse(stdout)).toMatchObject({ success: true })
  },
  30_000,
)

it.each(['node', process.execPath])(
  'qualifies binding lifecycle on actual runtime %s with package-built CJS and ESM',
  async (runtime) => {
    // Given
    const driver = join(import.meta.dir, 'mdx-isolate-binding-runtime.mjs')
    // When
    for (const format of ['cjs', 'esm']) {
      const { stdout, stderr } = await execute(runtime, [driver, format], {
        timeout: 25_000,
        maxBuffer: 1024 * 1024,
      })
      // Then
      expect(stderr).toBe('')
      expect(JSON.parse(stdout)).toMatchObject({ success: true, format })
    }
  },
  60_000,
)

it('keeps original current-H rejection on actual Bun with the package-built owner', async () => {
  // Given
  const driver = join(import.meta.dir, 'mdx-isolate-runtime.mjs')
  // When
  const execution = execute(process.execPath, [driver, 'esm', 'current-h'], {
    timeout: 25_000,
    maxBuffer: 1024 * 1024,
  })
  // Then
  await expect(execution).resolves.toMatchObject({ stderr: '' })
}, 30_000)
