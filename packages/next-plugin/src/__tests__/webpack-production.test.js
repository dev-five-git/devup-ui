import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

/**
 * @param {{data: {active: readonly number[], pids: readonly number[]}}} tree
 * @param {string} output
 * @param {NodeJS.Platform} platform
 */
function assertTreeExit(tree, output, platform = process.platform) {
  expect(tree.data.active).toEqual([])
  if (platform === 'win32') {
    expect(tree.data.pids.length).toBeGreaterThan(1)
    for (const match of output.matchAll(/W65-FAULT-PID (\d+)/g))
      expect(tree.data.pids).toContain(Number(match[1]))
  }
}

/**
 * Model only the two historical fixture IDs; never reach the OS signal API.
 * @param {number | undefined} reusedPid
 * @param {() => void} assertion
 */
function withHistoricalPidLookup(reusedPid, assertion) {
  const originalKill = process.kill
  const unexpected = []
  process.kill = (pid, signal) => {
    if (signal !== 0 || ![101, 202].includes(pid)) {
      unexpected.push({ pid, signal })
      throw new RangeError('Unexpected historical PID lookup')
    }
    if (pid === reusedPid) return true
    throw Object.assign(new Error('Historical process has exited'), {
      code: 'ESRCH',
    })
  }
  try {
    assertion()
  } finally {
    process.kill = originalKill
    // The old toThrow probe must not swallow an unexpected PID or signal.
    expect(unexpected).toEqual([])
  }
}

it('accepts a Windows job receipt when a historical PID belongs to an unrelated process', () => {
  // Given a reaped job whose historical ID 101 now identifies an unrelated process.
  const tree = { data: { active: [], pids: [101, 202] } }
  const output = 'W65-FAULT-PID 202'
  withHistoricalPidLookup(101, () => {
    // When the same receipt assertion used by real Next scenarios runs as Windows.
    // Then unrelated global PID reuse must not invalidate completed job ownership.
    expect(() => assertTreeExit(tree, output, 'win32')).not.toThrow()
  })
})

it('accepts a Windows job receipt when all historical PIDs are absent', () => {
  // Given a reaped job with both observed IDs absent from the global PID table.
  const tree = { data: { active: [], pids: [101, 202] } }
  withHistoricalPidLookup(undefined, () => {
    // When the shared receipt assertion runs as Windows.
    // Then the completed job and observed fault descendant are accepted.
    expect(() =>
      assertTreeExit(tree, 'W65-FAULT-PID 202', 'win32'),
    ).not.toThrow()
  })
})

for (const { condition, active, pids, output } of [
  {
    condition: 'owned membership is still active',
    active: [101],
    pids: [101, 202],
    output: '',
  },
  {
    condition: 'only one process was observed',
    active: [],
    pids: [101],
    output: '',
  },
  {
    condition: 'a fault descendant was not observed',
    active: [],
    pids: [101, 202],
    output: 'W65-FAULT-PID 303',
  },
])
  it(`rejects a Windows job receipt when ${condition}`, () => {
    // Given an invalid job receipt, with historical numeric lookups all absent.
    const tree = { data: { active, pids } }
    withHistoricalPidLookup(undefined, () => {
      // When the shared receipt assertion runs as Windows.
      // Then the missing ownership evidence is rejected independently of PID reuse.
      expect(() => assertTreeExit(tree, output, 'win32')).toThrow()
    })
  })

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
      .filter((line) => line.startsWith('W65-LIFECYCLE '))
      .map((line) => JSON.parse(line.slice(14)))
    const tree = events.find((entry) => entry.event === 'tree_exit')
    const cleanup = events.find((entry) => entry.event === 'cleanup')
    assertTreeExit(tree, output)
    expect(cleanup.data.removed).toBe(true)
    expect(existsSync(cleanup.data.root)).toBe(false)
    if (fault) {
      expect(result.status).not.toBe(0)
      expect(output).not.toContain('webpack production proof passed')
      if (fault === 'build-timeout') {
        expect(output.match(/W65-FAULT-PID \d+/g)?.length).toBe(2)
        expect(output).toContain('Next build deadline exceeded')
      } else expect(output).toContain('W65-SERVER-PID ')
      if (fault === 'post-start-timeout')
        expect(tree.data.reason).toBe('deadline')
    } else {
      expect(output).toContain('webpack production proof passed')
      expect(result.status).toBe(0)
    }
  }, 130000)
