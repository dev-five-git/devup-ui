import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

for (const singleCss of ['0', '1'])
  it(`delivers actual HTML-linked CSS through cold/repeat/warm webpack builds (singleCss=${singleCss})`, () => {
    // Given identical physical server, edge, client and accepted-package fixtures.
    // When real production documents and their stylesheet URLs are requested.
    const result = spawnSync(
      'node',
      [join(import.meta.dir, 'webpack-owned.cjs')],
      {
        encoding: 'utf8',
        maxBuffer: 16 * 1024 * 1024,
        env: { ...process.env, DEVUP_NEXT_SINGLE_CSS: singleCss },
      },
    )
    // Then the driver verifies class-bound declarations and cache stability.
    console.info(result.stdout + result.stderr)
    expect(result.error).toBeUndefined()
    expect(result.status).toBe(0)
    expect(result.stdout).toContain('webpack production proof passed')
    expect(result.stdout).toContain('"event":"tree_exit"')
    expect(result.stdout).toContain('"active":[]')
    expect(result.stdout).toContain('"removed":true')
  }, 130000)

for (const fault of [
  '',
  'build-timeout',
  'post-start-failure',
  'post-start-timeout',
  'driver-death',
])
  it(`owns genuine Next webpack processes through ${fault || 'success'}`, () => {
    // Given fresh built packages and a physical Next app created by the driver.
    // When the installed Next CLI builds it through the real WASM/webpack loader.
    const result = spawnSync(
      'node',
      [join(import.meta.dir, 'webpack-owned.cjs'), fault],
      { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 },
    )
    // Then production compilation and real SSR/edge requests succeed.
    expect(result.error).toBeUndefined()
    const output = result.stdout + result.stderr
    console.info(output)
    const events = output
      .split(/\r?\n/)
      .filter((line) => line.startsWith('NEXT-EVENT '))
      .map((line) => JSON.parse(line.slice('NEXT-EVENT '.length)))
    const tree = events.find((entry) => entry.event === 'tree_exit')
    const cleanup = events.find((entry) => entry.event === 'cleanup')
    expect(tree.data.active).toEqual([])
    expect(cleanup.data.removed).toBe(true)
    expect(existsSync(cleanup.data.root)).toBe(false)
    if (process.platform === 'win32') {
      expect(tree.data.pids.length).toBeGreaterThan(1)
      for (const pid of tree.data.pids) {
        expect(() => process.kill(pid, 0)).toThrow()
      }
      for (const match of output.matchAll(/FAULT-PID (\d+)/g))
        expect(tree.data.pids).toContain(Number(match[1]))
    }
    if (fault) {
      expect(result.status).not.toBe(0)
      expect(output).not.toContain('webpack production proof passed')
      if (fault === 'build-timeout') {
        expect(output.match(/FAULT-PID \d+/g)?.length).toBe(2)
        expect(output).toContain('Next build deadline exceeded')
      } else expect(output).toContain('SERVER-PID ')
      if (fault === 'post-start-timeout')
        expect(tree.data.reason).toBe('deadline')
    } else {
      expect(output).toContain('webpack production proof passed')
      expect(result.status).toBe(0)
    }
  }, 130000)
