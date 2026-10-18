import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

for (const mode of ['production', 'development']) {
  it(`qualifies genuine Next webpack ${mode} reservation and HTTP output`, () => {
    // Given: a unique physical app using the unchanged installed delegation.
    const driver = join(import.meta.dirname, 'native-webpack-run.cjs')
    // When: actual Next builds/serves the app (and dev revises two files).
    const result = spawnSync('node', [driver, mode], {
      encoding: 'utf8',
      timeout: 300000,
      maxBuffer: 32 * 1024 * 1024,
    })
    // Then: the native driver has checked engine receipts and HTTP assets.
    expect(result.error).toBeUndefined()
    expect({
      status: result.status,
      stderr: result.stderr,
      stdout: result.stdout,
    }).toEqual({
      status: 0,
      stderr: expect.any(String),
      stdout: expect.any(String),
    })
  }, 310000)
}
