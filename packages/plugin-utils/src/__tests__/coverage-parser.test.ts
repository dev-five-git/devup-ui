import { expect, it } from 'bun:test'

import { CoverageError, parseLcov } from '../../../../test-harness/lcov'

const complete =
  'TN:\nSF:src\\a.ts\nFN:2,first\nFNDA:1,first\nFNF:1\nFNH:1\nDA:2,1\nLF:1\nLH:1\nBRDA:2,0,0,-\nBRDA:2,0,1,1\nBRF:2\nBRH:1\nend_of_record\n'

it('retains source, function and branch identities when LCOV is parsed', () => {
  const [record] = parseLcov(complete)
  expect(record?.file).toBe('src/a.ts')
  expect(record?.functions.get('first')).toEqual({ line: 2, hits: 1 })
  expect([...(record?.branches ?? [])]).toEqual([
    ['2,0,0', 0],
    ['2,0,1', 1],
  ])
})

it('accepts Bun function totals when no FN identities are emitted', () => {
  const [record] = parseLcov(
    'SF:a.ts\nFNF:2\nFNH:1\nDA:1,1\nLF:1\nLH:1\nend_of_record',
  )
  expect(record?.foundFunctions).toBe(2)
  expect(record?.hitFunctions).toBe(1)
})

it.each([
  '',
  'TN:',
  'DA:1,1',
  'SF:a.ts',
  'SF:a.ts\nSF:b.ts',
  complete.replace('FNF:1', 'FNF:-1'),
  complete.replace('FNF:1', 'FNF:9007199254740992'),
  complete.replace('FNF:1', 'FNF:1\nFNF:1'),
  complete.replace('FNH:1', 'FNH:2'),
  complete.replace('LF:1', 'LF:2'),
  complete.replace('LH:1', 'LH:0'),
  complete.replace('FN:2,first\n', ''),
  complete.replace('FN:2,first', 'FN:2,first\nFN:3,first'),
  complete.replace('FN:2,first', 'FN:2,'),
  complete.replace('DA:2,1', 'DA:2,1,checksum'),
  complete.replace('DA:2,1', 'DA:2,1\nDA:2,1'),
  complete.replace('BRDA:2,0,0,-', 'BRDA:2,0,0,-\nBRDA:2,0,0,1'),
  complete.replace('BRF:2', 'BRF:1'),
  complete.replace('BRH:1', 'BRH:2'),
  complete.replace('FNDA:1,first', 'FNDA:0,first'),
  complete.replace('FNF:1', 'FNF:2'),
  complete.replace('FNF:1\n', ''),
  complete.replace('end_of_record', 'unknown:1'),
])('rejects malformed coverage when the report is %s', (report) => {
  expect(() => parseLcov(report)).toThrow(CoverageError)
})
