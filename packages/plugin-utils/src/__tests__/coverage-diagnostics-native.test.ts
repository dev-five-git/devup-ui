import { expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import { nativeFixture } from '../../../../test-harness/native-fixture'

it('reports both genuine partial sources with exact native zero rows and counts beside a complete sibling', async () => {
  const fixture = nativeFixture({
    'first.ts':
      'export function used(){return 1}\nexport function missed(){\n return 2\n}\n',
    'second.ts':
      'export function used(){return 3}\nexport function missed(){\n return 4\n}\nexport function alsoMissed(){\n return 5\n}\n',
    'complete.ts': 'export function used(){return 6}\n',
    'source.test.ts':
      "import {it,expect} from 'bun:test';import {used as first} from './first';import {used as second} from './second';import {used as complete} from './complete';it('actual',()=>{expect(first()).toBe(1);expect(second()).toBe(3);expect(complete()).toBe(6)})",
  })
  try {
    const result = await fixture.run()
    expect(result.code).toBe(1)
    expect(result.stderr).toMatch(/^\s*1 pass$/m)
    expect(result.stderr).toMatch(/^\s*0 fail$/m)
    expect(result.sources.map((source) => source.file).sort()).toEqual([
      'complete.ts',
      'first.ts',
      'second.ts',
    ])
    const expected = ['first.ts', 'second.ts'].map((file) => {
      const source = result.sources.find((source) => source.file === file)
      if (!source) throw new CoverageError(`missing native witness ${file}`)
      const zeroHitLines = [...source.lines]
        .filter(([, hits]) => hits === 0)
        .map(([line]) => line)
        .sort((a, b) => a - b)
      expect(zeroHitLines.length).toBeGreaterThan(0)
      expect(source.hitFunctions).toBeLessThan(source.foundFunctions)
      return {
        file,
        zeroHitLines,
        FNH: source.hitFunctions,
        FNF: source.foundFunctions,
        LH: [...source.lines.values()].filter((hits) => hits > 0).length,
        LF: source.lines.size,
      }
    })
    let failure: CoverageError | undefined
    try {
      auditCoverage(result.sources, ['first.ts', 'second.ts', 'complete.ts'])
    } catch (error) {
      if (!(error instanceof CoverageError)) throw error
      failure = error
    }
    if (!failure) throw new CoverageError('expected partial native rejection')
    expect(
      JSON.parse(failure.detail.slice('below 100% lines/functions: '.length)),
    ).toEqual(expected)
    expect(() =>
      auditCoverage(
        result.sources.filter((source) => source.file === 'complete.ts'),
        ['complete.ts'],
      ),
    ).not.toThrow()
  } finally {
    fixture.dispose()
  }
}, 120000)

it('rejects empty duplicate and missing native source inventories without modifying parsed observations', () => {
  const raw = 'SF:a.ts\nFNF:0\nFNH:0\nLF:0\nLH:0\nend_of_record\n'
  const sources = parseLcov(raw)
  expect(() => auditCoverage([], [])).toThrow('empty native report')
  expect(() => auditCoverage([...sources, ...sources], [])).toThrow(
    'duplicate or empty source',
  )
  expect(() => auditCoverage(sources, ['missing.ts'])).toThrow(
    'missing source missing.ts',
  )
  expect(() => auditCoverage(sources, ['a.ts'])).not.toThrow()
  expect(sources).toEqual(parseLcov(raw))
})
