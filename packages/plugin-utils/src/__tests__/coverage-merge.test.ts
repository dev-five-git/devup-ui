import { expect, it } from 'bun:test'

import {
  auditCoverage,
  coverageTable,
  mergeCoverage,
  writeLcov,
} from '../../../../test-harness/coverage'
import { CoverageError, parseLcov } from '../../../../test-harness/lcov'

const anonymous = (hitFunctions: number, hitLine: number) =>
  parseLcov(
    `SF:a.ts\nFNF:2\nFNH:${hitFunctions}\nDA:1,${hitLine}\nLF:1\nLH:${hitLine > 0 ? 1 : 0}\nend_of_record`,
  )
const named = (first: number, second: number) =>
  parseLcov(
    `SF:a.ts\nFN:1,first\nFN:2,second\nFNDA:${first},first\nFNDA:${second},second\nFNF:2\nFNH:${Number(first > 0) + Number(second > 0)}\nDA:1,${first}\nDA:2,${second}\nLF:2\nLH:${Number(first > 0) + Number(second > 0)}\nBRDA:1,0,0,${first}\nBRF:1\nBRH:${Number(first > 0)}\nend_of_record`,
  )

it('refuses named source identities without producer binding and preserves their observation serialization', () => {
  const reports = [named(1, 0), named(0, 1)] as const
  expect(() => mergeCoverage(reports)).toThrow(CoverageError)
  expect(parseLcov(writeLcov(reports[0]))).toEqual(reports[0])
  expect(reports[0]?.[0]?.functions.get('second')?.hits).toBe(0)
})

it('refuses invented function unions when Bun has only partial totals', () => {
  expect(() => mergeCoverage([anonymous(1, 0), anonymous(1, 1)])).toThrow(
    CoverageError,
  )
  expect(coverageTable(anonymous(1, 1))).toContain('a.ts | 50.00 | 100.00')
})

it('refuses summary-only completeness when another process covers an uncovered line', () => {
  expect(() =>
    auditCoverage(mergeCoverage([anonymous(2, 0), anonymous(1, 1)]), ['a.ts']),
  ).toThrow(CoverageError)
})

it('rejects the one uncovered source when other files are completely covered', () => {
  const complete = anonymous(2, 1)
  const partial = parseLcov(
    writeLcov(anonymous(2, 0)).replace('SF:a.ts', 'SF:b.ts'),
  )
  const merged = [...complete, ...partial]
  expect(() => auditCoverage(merged, ['a.ts', 'b.ts'])).toThrow(CoverageError)
  expect(coverageTable(merged)).toContain('b.ts | 100.00 | 0.00 | 1')
})

it.each([
  () => auditCoverage([], []),
  () => auditCoverage(anonymous(2, 1), ['missing.ts']),
  () => mergeCoverage([[...anonymous(2, 1), ...anonymous(2, 1)]]),
  () => mergeCoverage([named(1, 0), anonymous(1, 1)]),
  () =>
    auditCoverage(
      mergeCoverage([
        anonymous(2, 1),
        parseLcov(writeLcov(anonymous(1, 1)).replace('FNF:2', 'FNF:3')),
      ]),
      ['a.ts'],
    ),
  () =>
    mergeCoverage([
      named(1, 0),
      parseLcov(writeLcov(named(0, 1)).replace('FN:2,second', 'FN:3,second')),
    ]),
  () =>
    mergeCoverage([
      named(1, 0),
      parseLcov(writeLcov(named(0, 1)).replaceAll('second', 'third')),
    ]),
])('fails closed when inventory or identity evidence is invalid', (action) => {
  expect(action).toThrow(CoverageError)
})

it('reports vacuous line and function coverage when a loaded module has no executable definitions', () => {
  const sources = parseLcov(
    'SF:empty.ts\nFNF:0\nFNH:0\nLF:0\nLH:0\nend_of_record',
  )
  expect(coverageTable(sources)).toContain('empty.ts | 100.00 | 100.00')
})

it('refuses coarse-span deletion when import-only identity has no actual producer proof', () => {
  const executed = anonymous(2, 1)
  const imported = parseLcov(
    'SF:a.ts\nFNF:2\nFNH:0\nDA:1,0\nDA:2,0\nLF:2\nLH:0\nend_of_record',
  )
  expect(() => mergeCoverage([executed, imported])).toThrow(CoverageError)
  expect(imported[0]?.lines.has(2)).toBe(true)
})

it('retains a genuinely uncovered executable line when all processes report it', () => {
  const report = parseLcov(
    'SF:a.ts\nFNF:2\nFNH:2\nDA:1,1\nDA:2,0\nLF:2\nLH:1\nend_of_record',
  )
  expect(() =>
    auditCoverage(mergeCoverage([report, report]), ['a.ts']),
  ).toThrow(CoverageError)
})
