import { expect, it } from 'bun:test'

import { auditCoverage, mergeCoverage } from '../../../../test-harness/coverage'
import { CoverageError, parseLcov } from '../../../../test-harness/lcov'

const oracleA =
  'SF:a.ts\nFNF:2\nFNH:2\nDA:1,1\nDA:2,0\nLF:2\nLH:1\nend_of_record'
const oracleB =
  'SF:a.ts\nFNF:2\nFNH:1\nDA:1,1\nDA:3,0\nLF:2\nLH:1\nend_of_record'

it.each([
  [oracleA, oracleB],
  [oracleB, oracleA],
])(
  'refuses false100 when anonymous observations have divergent zero-hit universes',
  (first, second) => {
    const reports = [parseLcov(first), parseLcov(second)]
    expect(() => auditCoverage(mergeCoverage(reports), ['a.ts'])).toThrow(
      CoverageError,
    )
  },
)

it('refuses an invented function union when identical DA keys hide incompatible anonymous inventories', () => {
  const first = parseLcov(
    'SF:a.ts\nFNF:2\nFNH:2\nDA:1,1\nLF:1\nLH:1\nend_of_record',
  )
  const second = parseLcov(
    'SF:a.ts\nFNF:2\nFNH:1\nDA:1,1\nLF:1\nLH:1\nend_of_record',
  )
  expect(() => auditCoverage(mergeCoverage([first, second]), ['a.ts'])).toThrow(
    CoverageError,
  )
})

it('refuses unknown union cardinality when both anonymous observations claim complete coverage', () => {
  const report = parseLcov(
    'SF:a.ts\nFNF:2\nFNH:2\nDA:1,1\nLF:1\nLH:1\nend_of_record',
  )
  expect(() => mergeCoverage([report, report])).toThrow(CoverageError)
})
