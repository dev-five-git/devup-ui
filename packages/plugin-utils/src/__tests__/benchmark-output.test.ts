import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { evaluate } from '../../../../benchmark-gate.js'
import { measureOutput } from '../../../../benchmark-output.js'

describe('benchmark output measurement', () => {
  let directory: string

  beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'devup-benchmark-output-'))
  })

  afterEach(() => {
    rmSync(directory, { recursive: true, force: true })
  })

  function file(path: string, bytes: number) {
    const destination = join(directory, path)
    mkdirSync(dirname(destination), { recursive: true })
    writeFileSync(destination, Buffer.alloc(bytes, 'x'))
  }

  it.each(['static/css/page.css', 'static/chunks/nested/page.css'])(
    'counts only shipped CSS when Next emits %s alongside compiler caches',
    (path) => {
      file('.next/' + path, 494)
      file('.next/cache/devup-ui_build/page.css', 450)
      file('.next/cache/devup-ui_build/base.css', 68)
      file('.next/server/app/page.css', 83)
      file('.next/static/chunks/app.js', 101)

      const result = measureOutput(directory)

      expect(result).toEqual({ bytes: 1196, cssBytes: 494 })
    },
  )

  it('prefers dist and counts client CSS when vinext also emits a Next stub', () => {
    file('.next/static/css/stub.css', 999)
    file('dist/client/assets/page.css', 31)
    file('dist/client/assets/nested/shared.css', 17)
    file('dist/client/assets/app.js', 101)
    file('dist/server/assets/page.css', 67)
    file('dist/server/app.js', 83)

    const result = measureOutput(directory)

    expect(result).toEqual({ bytes: 299, cssBytes: 48 })
  })

  it('rejects measurement when neither output directory exists', () => {
    expect(() => measureOutput(directory)).toThrow()
  })

  it.each(['.next', 'dist'])(
    'rejects measurement when %s has no required browser-assets directory',
    (output) => {
      file(output + '/server/page.css', 123)

      expect(() => measureOutput(directory)).toThrow()
    },
  )

  it('does not fall back to Next when the preferred dist lacks client assets', () => {
    file('.next/static/css/page.css', 494)
    file('dist/server/page.css', 123)

    expect(() => measureOutput(directory)).toThrow()
  })

  it.each(['.next/static', 'dist/client'])(
    'counts zero CSS when the existing browser-assets directory %s is empty',
    (browser) => {
      mkdirSync(join(directory, browser), { recursive: true })

      const result = measureOutput(directory)

      expect(result).toEqual({ bytes: 0, cssBytes: 0 })
    },
  )

  it.each(['.next/static', 'dist/client'])(
    'counts zero CSS when %s contains only non-CSS assets',
    (browser) => {
      file(browser + '/assets/app.js', 41)
      file(browser + '/assets/page.css.map', 23)

      const result = measureOutput(directory)

      expect(result).toEqual({ bytes: 64, cssBytes: 0 })
    },
  )

  it.each([
    { cssBytes: 494, failed: false },
    { cssBytes: 526, failed: false },
    { cssBytes: 527, failed: true },
  ])(
    'enforces shipped CSS headroom when the asset is $cssBytes bytes',
    ({ cssBytes, failed }) => {
      file('.next/static/css/page.css', cssBytes)
      file('.next/cache/devup-ui_build/page.css', 450)
      const baseline = {
        samples: 3,
        rows: {
          'devup-ui': {
            control: 'tailwind',
            ratio: 1,
            timeTolerance: 0.1,
            bytes: 1000,
            cssBytes: 494,
          },
        },
      }
      const run = {
        samples: 3,
        rows: {
          tailwind: { median: 1 },
          'devup-ui': { median: 1, ...measureOutput(directory) },
        },
      }

      const evaluation = evaluate(run, baseline)

      expect(evaluation.failed).toBe(failed)
      expect(evaluation.results).toEqual([
        {
          target: 'devup-ui',
          failures: failed
            ? ['CSS is 527 bytes, over the limit 526 (baseline 494)']
            : [],
        },
      ])
    },
  )
})
