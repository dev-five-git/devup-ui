import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { beforeAll, describe, expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import {
  CoverageError,
  parseLcov,
  type SourceCoverage,
} from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { reconstructNative } from '../../../../test-harness/producer-native'
import { captureProcess } from '../../../../test-harness/producer-process'

describe('genuine native coverage diagnostics', () => {
  let sources: readonly SourceCoverage[] = []
  let expected: readonly object[] = []

  beforeAll(async () => {
    // Given: two partially exercised modules and an independently complete control.
    const root = mkdtempSync(join(tmpdir(), 'devup-coverage-diagnostics-'))
    try {
      const config = join(root, 'bunfig.toml')
      writeFileSync(
        config,
        '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
      )
      writeFileSync(
        join(root, 'first.ts'),
        'export function used() { return 1 }\nexport function missed() {\n  return 2\n}\n',
      )
      writeFileSync(
        join(root, 'second.ts'),
        'export function used() { return 3 }\nexport function missed() {\n  return 4\n}\nexport function alsoMissed() {\n  return 5\n}\n',
      )
      writeFileSync(
        join(root, 'complete.ts'),
        'export function used() { return 6 }\n',
      )
      writeFileSync(
        join(root, 'source.test.ts'),
        "import {it,expect} from 'bun:test';import {used as first} from './first';import {used as second} from './second';import {used as complete} from './complete';it('actual',()=>{expect(first()).toBe(1);expect(second()).toBe(3);expect(complete()).toBe(6)})",
      )
      const result = await captureProcess({
        root,
        config,
        coverage: join(root, 'coverage'),
        group: 'isolated',
        files: ['source.test.ts'],
        executable: process.execPath,
      })
      expect(result.value.status).toBe(0)
      const official = parseLcov(result.value.lcov)
      sources = VerifiedSourceCoverage.bind(result, root).filter((source) =>
        ['first.ts', 'second.ts', 'complete.ts'].includes(source.file),
      )
      expect(sources.map((source) => source.file).sort()).toEqual([
        'complete.ts',
        'first.ts',
        'second.ts',
      ])
      expected = ['first.ts', 'second.ts'].map((file) => {
        const source = official.find((item) => item.file === file)
        const capture = result.value.captures.find(
          (item) => item.url === join(root, file),
        )
        const bound = sources.find((item) => item.file === file)
        if (!source || !capture || !bound)
          throw new CoverageError('missing actual diagnostics fixture')
        const native = reconstructNative(capture, source)
        const zeroHitLines = [...source.lines]
          .filter(([, hits]) => hits === 0)
          .map(([line]) => line)
          .sort((a, b) => a - b)
        expect(zeroHitLines.length).toBeGreaterThan(0)
        expect(source.hitFunctions).toBeLessThan(source.foundFunctions)
        expect(native.functions.length).toBe(source.foundFunctions)
        expect(native.functions.filter((fn) => fn.hasExecuted).length).toBe(
          source.hitFunctions,
        )
        expect([...native.lines].sort(([a], [b]) => a - b)).toEqual(
          [...source.lines].sort(([a], [b]) => a - b),
        )
        expect([...bound.lines]).toEqual([...source.lines])
        expect([bound.hitFunctions, bound.foundFunctions]).toEqual([
          source.hitFunctions,
          source.foundFunctions,
        ])
        expect(bound.functions.size).toBe(0)
        return {
          file,
          zeroHitLines,
          FNH: source.hitFunctions,
          FNF: source.foundFunctions,
          LH: [...source.lines.values()].filter((hits) => hits > 0).length,
          LF: source.lines.size,
          functionNames: 'unavailable from verified producer',
        }
      })
      console.info(
        JSON.stringify({
          bun: result.value.bun,
          revision: result.value.revision,
          pid: result.value.pid,
          official: result.value.lcov,
          captures: result.value.captures.filter((capture) =>
            ['first.ts', 'second.ts', 'complete.ts'].some(
              (file) => capture.url === join(root, file),
            ),
          ),
          expected,
        }),
      )
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  }, 120000)

  it('reports every partial source with exact zero rows and native counts', () => {
    // When: audit the real issued inventory, including the complete control.
    let failure: CoverageError | undefined
    try {
      auditCoverage(sources, ['first.ts', 'second.ts', 'complete.ts'])
    } catch (error) {
      if (!(error instanceof CoverageError)) throw error
      failure = error
    }
    // Then: one structured detail contains precisely the two failing records.
    if (!failure) throw new CoverageError('expected partial native failure')
    const prefix = 'below 100% lines/functions: '
    expect(failure.detail.startsWith(prefix)).toBe(true)
    const detail: unknown = JSON.parse(failure.detail.slice(prefix.length))
    expect(detail).toEqual(expected)
  })

  it('accepts the genuinely complete source selection', () => {
    const complete = sources.filter((source) => source.file === 'complete.ts')
    expect(() => auditCoverage(complete, ['complete.ts'])).not.toThrow()
  })
})
