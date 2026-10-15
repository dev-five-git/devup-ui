import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'

import storedBaseline from '../../../../benchmark-baseline.json'
import { nextCssBaseline } from '../../../../benchmark-gate.js'

const baseline = {
  ...storedBaseline,
  revision: 'baseline-only',
  rows: {
    ...storedBaseline.rows,
    'devup-ui': { ...storedBaseline.rows['devup-ui'], cssBytes: 562 },
  },
}

// Deliberately disagree with every stored field, including the history.
const run = {
  revision: 'run-only',
  samples: 9,
  rows: Object.fromEntries(
    Object.entries(baseline.rows).map(([target, row]) => [
      target,
      {
        control: 'different-control',
        ratio: row.ratio + 1,
        calibration: [9],
        timeTolerance: row.timeTolerance + 1,
        bytes: row.bytes + 123,
        cssBytes: target === 'devup-ui' ? 494 : row.cssBytes + 456,
        samples: [99],
        median: 99,
        min: 98,
        max: 100,
      },
    ]),
  ),
}
const expected = {
  ...baseline,
  rows: {
    ...baseline.rows,
    'devup-ui': { ...baseline.rows['devup-ui'], cssBytes: 494 },
  },
}

describe('CSS-only benchmark baseline', () => {
  it('preserves every other field when the run disagrees with the baseline', () => {
    const previous = structuredClone(baseline)
    const input = structuredClone(run)

    const result = nextCssBaseline(input, previous)

    expect(result).toEqual(expected)
    expect(previous).toEqual(baseline)
    expect(input).toEqual(run)
  })

  it('preserves the baseline when CSS bytes are unchanged', () => {
    const input = {
      ...run,
      rows: {
        ...run.rows,
        'devup-ui': { cssBytes: baseline.rows['devup-ui'].cssBytes },
      },
    }

    const result = nextCssBaseline(input, baseline)

    expect(result).toEqual(baseline)
  })

  it('rejects a baseline when the existing per-file row is absent', () => {
    const previous = { samples: 3, rows: {} }

    expect(() => nextCssBaseline(run, previous)).toThrow(TypeError)
  })

  it('rejects a run when the per-file measurement is absent', () => {
    const input = { samples: 3, rows: {} }

    expect(() => nextCssBaseline(input, baseline)).toThrow(TypeError)
  })
})

describe('CSS-only baseline CLI', () => {
  it.each([
    { args: ['--write-css-baseline', 'devup-ui'], success: true },
    { args: ['--write-css-baseline', 'devup-ui-turbo'], success: false },
    { args: ['--write-css-baseline'], success: false },
    {
      args: ['--write-css-baseline', 'devup-ui', '--write-baseline'],
      success: false,
    },
    {
      args: ['--write-css-baseline', 'devup-ui', '--write-results'],
      success: false,
    },
  ])(
    'writes only per-file CSS when invoked with $args',
    ({ args, success }) => {
      const directory = mkdtempSync(join(tmpdir(), 'devup-css-baseline-'))
      const stored = JSON.stringify(baseline, null, 2) + '\n'
      try {
        writeFileSync(join(directory, 'benchmark-baseline.json'), stored)
        writeFileSync(join(directory, 'benchmark-manifest.json'), '{}\n')
        writeFileSync(
          join(directory, 'benchmark-run.json'),
          JSON.stringify(run),
        )
        writeFileSync(join(directory, 'benchmark-results.json'), '{}\n')

        const result = spawnSync(
          process.execPath,
          [join(import.meta.dir, '../../../../benchmark-check.js'), ...args],
          { cwd: directory, encoding: 'utf8', timeout: 10_000 },
        )

        expect(result.error).toBeUndefined()
        expect(result.status).toBe(success ? 0 : 1)
        expect(
          readFileSync(join(directory, 'benchmark-baseline.json'), 'utf8'),
        ).toBe(success ? JSON.stringify(expected, null, 2) + '\n' : stored)
        expect(
          readFileSync(join(directory, 'benchmark-results.json'), 'utf8'),
        ).toBe('{}\n')
      } finally {
        rmSync(directory, { recursive: true, force: true })
      }
    },
  )
})
