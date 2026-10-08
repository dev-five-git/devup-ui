import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

it('delivers related CSS and isolates native same-filesystem/default-output and watch generations', () => {
  // Given
  const driver = resolve(import.meta.dir, '../../generation-regression.mjs')
  // When
  const result = spawnSync(process.execPath, [driver], {
    encoding: 'utf8',
    timeout: 180000,
    env: process.env,
  })
  // Then
  if (result.status !== 0)
    process.stderr.write(result.stdout + '\n' + result.stderr)
  expect(result.error).toBeUndefined()
  expect(result.status, result.stderr).toBe(0)
}, 190000)

it('delivers inherited theme variables in HTML-linked native CSS in both modes and edited generations', () => {
  // Given
  const driver = resolve(
    import.meta.dir,
    '../../theme-generation-regression.mjs',
  )
  // When
  const result = spawnSync(process.execPath, [driver], {
    encoding: 'utf8',
    timeout: 180000,
    env: process.env,
  })
  // Then
  if (result.status !== 0)
    process.stderr.write(result.stdout + '\n' + result.stderr)
  expect(result.error).toBeUndefined()
  expect(result.status, result.stderr).toBe(0)
}, 190000)
