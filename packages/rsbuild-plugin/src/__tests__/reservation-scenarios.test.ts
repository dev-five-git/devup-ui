import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { record } from './reservation-receipt'

const require = createRequire(import.meta.url)
const fixture: {
  prepare(root: string): void
} = require('./reservation-fixture.cjs')

function run(root: string, scenario: string) {
  const receipt = join(root, 'result.json')
  const child = spawnSync(
    'node',
    [
      join(import.meta.dir, 'reservation-scenarios.cjs'),
      root,
      scenario,
      receipt,
    ],
    { encoding: 'utf8', timeout: 60000 },
  )
  assert.equal(child.status, 0, child.stdout + child.stderr)
  const result: unknown = JSON.parse(readFileSync(receipt, 'utf8'))
  record(result)
  assert(
    Array.isArray(result.batches) &&
      Array.isArray(result.outputs) &&
      Array.isArray(result.events) &&
      Array.isArray(result.hoist),
  )
  return {
    count: result.count,
    setups: result.setups,
    css: result.css,
    entries: result.entries,
    compilations: result.compilations,
    assets: result.assets,
    classMap: result.classMap,
    fileMap: result.fileMap,
    batches: result.batches.map((batch: unknown) => batch),
    outputs: result.outputs.map((output: unknown) => output),
    events: result.events.map((event: unknown) => event),
    hoist: result.hoist.map((threshold: unknown) => threshold),
  }
}

for (const scenario of ['atom', 'noop', 'multi', 'static', 'opaque']) {
  it(`preserves native ${scenario} semantics with physical reservation`, () => {
    // Given real Rsbuild inputs and public environment/native entry configuration.
    const root = realpathSync.native(
      mkdtempSync(join(tmpdir(), 'devup-rsbuild-scenario-')),
    )
    try {
      // When the installed Rsbuild/Rspack production compiler builds them.
      const result = run(root, scenario)
      // Then only genuinely outside-D9 opaque input has a residual slot.
      expect(result.count).toBe(scenario === 'opaque' ? 1 : 0)
      expect(result.events[0]).toBe('reserve')
      expect(result.setups).toBe(1)
      expect(result.batches).toHaveLength(1)
      if (scenario === 'atom') {
        expect(result.hoist).toEqual([2])
        expect(result.css).toContain('27px')
      }
      if (scenario === 'noop') expect(result.hoist).toEqual([])
      if (scenario === 'multi') {
        expect(result.css).toContain('olive')
        expect(result.css).toContain('coral')
        expect(JSON.stringify(result.batches[0])).toContain('outside-a')
        expect(JSON.stringify(result.batches[0])).toContain('outside-b')
      }
      if (scenario === 'static')
        expect(JSON.stringify(result.batches[0])).toContain('external')
      if (scenario === 'opaque') {
        expect(result.entries).toBe(1)
        expect(result.compilations).toBe(1)
        expect(JSON.stringify(result.batches[0])).not.toContain('external')
        expect(JSON.stringify(result.outputs)).toContain('--var-')
      }
      console.info(
        JSON.stringify({
          scenario,
          count: result.count,
          batches: result.batches.length,
          compilations: result.compilations,
        }),
      )
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  }, 120000)
}

it('ignores cold/warm own output and stale unrelated output trees', () => {
  // Given one source inventory, preserved own output, and stale generated sources.
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-rsbuild-warm-')),
  )
  try {
    const cold = run(root, 'scope')
    const source =
      'import {css} from "@devup-ui/react"; console.log(css({color:"chartreuse"}))'
    for (const directory of ['out', 'dist', '.next/server']) {
      const file = join(root, directory, 'stale.js')
      mkdirSync(join(root, directory), { recursive: true })
      writeFileSync(file, source)
    }
    // When a fresh native process builds the unchanged source.
    const warm = run(root, 'scope')
    // Then actual emitted assets, naming maps and inventory stay identical.
    expect(warm.assets).toEqual(cold.assets)
    expect(warm.classMap).toEqual(cold.classMap)
    expect(warm.fileMap).toEqual(cold.fileMap)
    expect(warm.batches).toEqual(cold.batches)
    expect(warm.count).toBe(0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)

it.each(['root', 'multi-root'])(
  'keeps exact static %s entries and closure independent of output/config/sibling trees',
  (scenario) => {
    // Given: actual root-level native entries and their physical import closure.
    const root = realpathSync.native(
      mkdtempSync(join(tmpdir(), 'devup-rsbuild-exact-')),
    )
    try {
      const cold = run(root, scenario)
      const warm = run(root, scenario)
      expect(warm.assets).toEqual(cold.assets)
      expect(warm.batches).toEqual(cold.batches)
      for (const directory of ['dist', '.next/server', 'unused']) {
        mkdirSync(join(root, directory), { recursive: true })
        writeFileSync(
          join(root, directory, 'stale.js'),
          'import {css} from "@devup-ui/react"; console.log(css({color:"chartreuse"}))',
        )
      }
      writeFileSync(
        join(root, 'rsbuild.config.js'),
        'import {css} from "@devup-ui/react"; export default css({margin:"99px"})',
      )
      // When: fresh real compilers rebuild the same known entry union.
      const stale = run(root, scenario)
      // Then: actual output/allocators and all discovery/extraction work remain equal.
      expect(stale.assets).toEqual(cold.assets)
      expect(stale.css).toEqual(cold.css)
      expect(stale.fileMap).toEqual(cold.fileMap)
      expect(stale.classMap).toEqual(cold.classMap)
      expect(stale.batches).toEqual(cold.batches)
      expect(
        stale.outputs.map((output) => JSON.stringify(output)).sort(),
      ).toEqual(cold.outputs.map((output) => JSON.stringify(output)).sort())
      expect(stale.events).toEqual(cold.events)
      expect(stale.count).toBe(0)
      expect(stale.batches).toHaveLength(1)
      if (scenario === 'multi-root') {
        expect(JSON.stringify(stale.batches[0])).toContain('outside-a')
        expect(JSON.stringify(stale.batches[0])).toContain('outside-b')
      } else {
        expect(JSON.stringify(stale.batches[0])).toContain('entry.css.ts')
      }
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  180000,
)

it('reports a reachable extraction error only through the real native transform', () => {
  // Given the same invalid source that is harmless while unreachable.
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-rsbuild-error-')),
  )
  try {
    fixture.prepare(root)
    // When native Rspack actually loads that source.
    const child = spawnSync(
      'node',
      [
        join(import.meta.dir, 'reservation-native.cjs'),
        root,
        'AB',
        'error',
        join(root, 'error.json'),
      ],
      { encoding: 'utf8', timeout: 60000 },
    )
    // Then its original located build-time diagnostic is published.
    expect(child.status).toBe(1)
    expect(child.stdout + child.stderr).toContain('invalid.js:1:')
    expect(child.stdout + child.stderr).toContain('cannot use')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
