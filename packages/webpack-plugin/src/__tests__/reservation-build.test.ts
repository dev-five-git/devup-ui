import assert from 'node:assert/strict'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { parseReservationReceipt } from './reservation-receipt'

function run(fixture: string, schedule: string, mode: string, cssMode: string) {
  const child = Bun.spawnSync(
    [
      'node',
      join(import.meta.dir, 'reservation-build.cjs'),
      fixture,
      schedule,
      mode,
      cssMode,
    ],
    { stdout: 'pipe', stderr: 'pipe', timeout: 45000 },
  )
  assert.equal(
    child.exitCode,
    0,
    child.stdout.toString() + child.stderr.toString(),
  )
  const receipt = parseReservationReceipt(child.stdout.toString())
  assert(receipt.final && Object.keys(receipt.final.assets).length > 0)
  return receipt
}

for (const cssMode of ['single', 'per-file']) {
  for (const mode of ['production', 'watch']) {
    it(`emits identical real webpack ${mode} output in ${cssMode} mode when loaders run AB or BA`, () => {
      // Given: identical physical IDs, fresh engines, reverse loader/write notification order.
      const fixture = mkdtempSync(join(tmpdir(), 'devup-webpack-slots-'))
      try {
        // When: native webpack runs, including a genuine suspended/resumed watch revision.
        const ab = run(fixture, 'AB', mode, cssMode)
        rmSync(fixture, { recursive: true, force: true })
        const ba = run(fixture, 'BA', mode, cssMode)
        // Then: compare genuine bytes BEFORE checking the initially inactive residual seam.
        expect(ba.final).toEqual(ab.final)
        expect(ab.final.css).toContain('red')
        expect(ab.final.css).toContain('blue')
        expect(ab.final.css).toContain('@keyframes')
        expect(ab.final.transformed['A.css.ts']).toMatch(/--[a-z0-9_-]+/)
        expect(ab.final.residual).toBe(0)
        expect(ab.reservations.length).toBeGreaterThanOrEqual(
          mode === 'watch' ? 2 : 1,
        )
        const initialBatch = ab.reservations[0]
        assert(initialBatch)
        expect(initialBatch.rows).toContain('src/0-Generated.js')
        expect(initialBatch.rows).toContain(
          'node_modules/@slot/ui/dist/index.js',
        )
        expect(initialBatch.rows).toContain('src/Invalid.js')
        expect(ab.final.canonical['src/child.js']).toBe('src/entry.js')
        if (mode === 'watch') {
          assert(ab.initial)
          for (const [id, slot] of Object.entries(ab.initial.files))
            expect(ab.final.files[id]).toEqual(slot)
          const batch = ab.reservations.find((item) => item.revision === 1)
          assert(batch)
          expect(batch.rows.indexOf('physical/A.js')).toBeLessThan(
            batch.rows.indexOf('physical/B.js'),
          )
        }
      } finally {
        rmSync(fixture, { recursive: true, force: true })
      }
    }, 100000)
  }
}

it('keeps reached-source prewarm first after scratch restoration with already reserved names', () => {
  // Given: static entries that main already prewarms.
  const fixture = mkdtempSync(join(tmpdir(), 'devup-webpack-prewarm-'))
  try {
    // When: the production compiler runs.
    const receipt = run(fixture, 'AB', 'prewarm', 'per-file')
    // Then: scratch precedes real prewarm, which inserts no class/file names.
    expect(receipt.events[0]).toBe('reserve')
    expect(receipt.events[1]).toBe('prewarm')
    const prewarm = receipt.extracts.filter((item) => item.prewarm)
    expect(prewarm.length).toBeGreaterThan(0)
    for (const item of prewarm) {
      expect(item.sameClasses).toBe(true)
      expect(item.sameFiles).toBe(true)
      expect(item.residual).toBe(0)
    }
    expect(receipt.compilations).toBe(1)
    expect(receipt.final.residual).toBe(0)
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}, 50000)

for (const mode of ['debug', 'atom', 'noop']) {
  it(`preserves ${mode} configuration and zero residuals in genuine production output`, () => {
    const fixture = mkdtempSync(join(tmpdir(), 'devup-webpack-config-'))
    try {
      const ab = run(fixture, 'AB', mode, 'per-file')
      rmSync(fixture, { recursive: true, force: true })
      const ba = run(fixture, 'BA', mode, 'per-file')
      expect(ba.final).toEqual(ab.final)
      expect(ab.final.residual).toBe(0)
      if (mode === 'debug') expect(ab.final.css).toContain('slot')
      if (mode === 'atom') {
        expect(ab.atomThresholds).toContain(2)
        expect(ab.atomThresholds.every((threshold) => threshold === 2)).toBe(
          true,
        )
      }
      if (mode === 'noop') {
        expect(ab.atomThresholds).toContain(undefined)
        expect(
          ab.atomThresholds.every((threshold) => threshold === undefined),
        ).toBe(true)
      }
    } finally {
      rmSync(fixture, { recursive: true, force: true })
    }
  }, 100000)
}

it('keeps watch ordinals append-only in a session and schedule-independent in a fresh compiler', () => {
  // Given: a first watch session that appends files after its initial inventory.
  const fixture = mkdtempSync(join(tmpdir(), 'devup-webpack-fresh-'))
  try {
    const first = run(fixture, 'AB', 'watch', 'per-file')
    assert(first.initial)
    // When: fresh compilers start in the same directory under both loader schedules.
    const restored = run(fixture, 'BA', 'watch', 'per-file')
    const repeated = run(fixture, 'AB', 'watch', 'per-file')
    // Then: no session reorders its own ordinals, and the starting inventory alone fixes the next session.
    for (const [id, slot] of Object.entries(first.initial.files))
      expect(first.final.files[id]).toEqual(slot)
    expect(repeated.final.files).toEqual(restored.final.files)
    expect(repeated.final.raw.classes).toEqual(restored.final.raw.classes)
    expect(restored.final.residual).toBe(0)
    expect(repeated.final.residual).toBe(0)
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}, 100000)

it('reports a reachable invalid source only through the genuine webpack transform', () => {
  const fixture = mkdtempSync(join(tmpdir(), 'devup-webpack-error-'))
  try {
    const child = Bun.spawnSync(
      [
        'node',
        join(import.meta.dir, 'reservation-build.cjs'),
        fixture,
        'AB',
        'invalid',
        'single',
      ],
      { stdout: 'pipe', stderr: 'pipe', timeout: 45000 },
    )
    expect(child.exitCode).toBe(1)
    expect(child.stderr.toString()).toContain('ERROR in ./src/Invalid.js')
    expect(child.stderr.toString()).toContain('props.unknown')
    expect(child.stderr.toString()).toContain('webpack-plugin/dist/loader.cjs')
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}, 50000)

it('lets webpack invoke a function entry exactly once in a one-pass production build', () => {
  const fixture = mkdtempSync(join(tmpdir(), 'devup-webpack-function-'))
  try {
    const receipt = run(fixture, 'AB', 'once', 'single')
    expect(receipt.entryCalls).toBe(1)
    expect(receipt.compilations).toBe(1)
    expect(receipt.final.residual).toBe(0)
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}, 50000)
