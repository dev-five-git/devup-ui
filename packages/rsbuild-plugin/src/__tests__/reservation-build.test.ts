import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { assertRetained, parseReceipt, record } from './reservation-receipt'

const require = createRequire(import.meta.url)
const fixture: {
  prepare(root: string, watch: boolean): void
} = require('./reservation-fixture.cjs')

function run(root: string, order: string, mode: string) {
  const receipt = join(root, `${order}.json`)
  const child = spawnSync(
    'node',
    [
      join(import.meta.dir, 'reservation-native.cjs'),
      root,
      order,
      mode,
      receipt,
    ],
    { encoding: 'utf8', timeout: 60000 },
  )
  assert.equal(child.status, 0, child.stdout + child.stderr)
  const value: unknown = JSON.parse(readFileSync(receipt, 'utf8'))
  return parseReceipt(value)
}

for (const mode of ['build', 'run', 'watch', 'dev']) {
  it(`emits identical real output when ${mode} extraction order reverses`, () => {
    // Given the same physical sources and fixed initial watch history.
    const root = realpathSync.native(
      mkdtempSync(join(tmpdir(), 'devup-rsbuild-reserve-')),
    )
    try {
      const revision = mode === 'watch' || mode === 'dev'
      fixture.prepare(root, revision)
      // When native Rspack reaches the modules in opposite orders.
      const ab = run(root, 'AB', mode)
      if (revision) rmSync(join(root, 'app'), { recursive: true })
      fixture.prepare(root, revision)
      const ba = run(root, 'BA', mode)
      // Then actual source and stylesheet bytes agree (no name projection).
      expect(ab.modules).toEqual(ba.modules)
      expect(ab.css).toEqual(ba.css)
      expect(ab.count).toBe(0)
      expect(ba.count).toBe(0)
      expect(ab.classMap).toEqual(ba.classMap)
      expect(ab.fileMap).toEqual(ba.fileMap)
      if (mode !== 'dev') expect(ab.assets).toEqual(ba.assets)
      for (const result of [ab, ba]) {
        if (mode === 'run') {
          expect(result.css).not.toContain('padding:19px')
          record(result.assets)
          const emittedCss = Object.entries(result.assets)
            .filter(([name]) => name.endsWith('.css'))
            .map(([, source]) => {
              assert(typeof source === 'string')
              return source
            })
          expect(emittedCss.length).toBeGreaterThan(0)
          expect(emittedCss.join('\n')).not.toMatch(/padding:\s*19px/)
        }
        expect(result.batches.length).toBeGreaterThan(0)
        expect(
          result.outputs.every(
            ({ before, after }) => before === 0 && after === 0,
          ),
        ).toBe(true)
        expect(result.entries).toBe(result.compilations)
        expect(result.beforeBuild).toBe(mode === 'build' ? 1 : 0)
        expect(result.css).toContain(
          mode === 'watch' ? 'darkmagenta' : revision ? 'crimson' : 'red',
        )
        expect(result.css).toContain('17px')
        expect(result.css).toContain('@keyframes')
        const final = result.batches.at(-1)
        assert(final)
        for (const batch of result.batches) {
          assert(
            typeof batch.classMap === 'string' &&
              typeof batch.fileMap === 'string',
          )
          assert(
            typeof result.classMap === 'string' &&
              typeof result.fileMap === 'string',
          )
          const oldClasses: unknown = JSON.parse(batch.classMap)
          const classes: unknown = JSON.parse(result.classMap)
          const oldFiles: unknown = JSON.parse(batch.fileMap)
          const files: unknown = JSON.parse(result.fileMap)
          assertRetained(oldClasses, classes)
          assertRetained(oldFiles, files)
        }
        expect(final.rows.map((row) => row.filename)).toContain(
          join(root, 'node_modules/included-ui/dist/index.js'),
        )
        expect(
          final.rows.some((row) => row.filename.endsWith('invalid.js')),
        ).toBe(true)
        expect(
          final.rows.map((row) => row.filename.replaceAll('\\', '/')),
        ).toEqual(
          final.rows.map((row) => row.filename.replaceAll('\\', '/')).sort(),
        )
        if (mode === 'build' || mode === 'run') {
          expect(result.events[0]).toBe('reserve')
        }
        if (mode === 'build') {
          expect(result.outputs[0]?.name).toBe(join(root, 'src/entry.js'))
        }
      }
      console.info(
        JSON.stringify({
          scenario: mode,
          residualAB: ab.count,
          residualBA: ba.count,
          batches: [ab.batches.length, ba.batches.length],
          compilations: [ab.compilations, ba.compilations],
        }),
      )
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  }, 120000)
}
