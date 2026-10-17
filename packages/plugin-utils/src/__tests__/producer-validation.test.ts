import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterAll, beforeAll, expect, it } from 'bun:test'

import { auditCoverage, mergeCoverage } from '../../../../test-harness/coverage'
import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import {
  array,
  boolean,
  integer,
  object,
  parseBlocks,
  parseProfiles,
  type ScriptCapture,
  text,
} from '../../../../test-harness/producer-data'
import { decodeMap } from '../../../../test-harness/producer-map'
import {
  reconstructNative,
  validateNative,
} from '../../../../test-harness/producer-native'
import {
  captureProcess,
  type ProducerResult,
} from '../../../../test-harness/producer-process'

const root = mkdtempSync(join(tmpdir(), 'devup-producer-validation-'))
let result: ProducerResult
let capture: ScriptCapture
beforeAll(async () => {
  const config = join(root, 'bunfig.toml')
  writeFileSync(
    config,
    '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
  )
  writeFileSync(
    join(root, 'source.ts'),
    'export function first() { return 1 }\nexport function second() { return 2 }\n',
  )
  writeFileSync(
    join(root, 'source.test.ts'),
    "import {it,expect} from 'bun:test';import {first,second} from './source';it('actual',()=>{expect(first()).toBe(1);expect(second()).toBe(2)})",
  )
  result = await captureProcess({
    root,
    config,
    coverage: join(root, 'coverage'),
    group: 'isolated',
    files: ['source.test.ts'],
    executable: process.execPath,
  })
  const source = result.value.captures.find(
    (capture) => capture.url === join(root, 'source.ts'),
  )
  if (!source) throw new CoverageError('missing actual validation fixture')
  capture = source
}, 120000)
afterAll(() => rmSync(root, { recursive: true, force: true }))

it('binds every actual native counted function when public synthetic and module entries are removed correctly', () => {
  const bound = VerifiedSourceCoverage.bind(result, root).find(
    (source) => source.file === 'source.ts',
  )
  expect(bound?.foundFunctions).toBe(2)
  expect(bound?.hitFunctions).toBe(2)
  expect(capture.profile?.functions.length).toBe(4)
})

it.each([
  () => object(null),
  () => object([]),
  () => text(1),
  () => integer(-1),
  () => integer(1.5),
  () => boolean('true'),
  () => array(null),
  () => parseProfiles([{}]),
  () => parseBlocks([{}]),
])('refuses malformed public data at its parsing boundary', (action) =>
  expect(action).toThrow(CoverageError),
)

it.each([
  () => reconstructNative({ ...capture, profile: undefined }),
  () => reconstructNative({ ...capture, blocks: [] }),
  () =>
    reconstructNative({ ...capture, blocks: [...capture.blocks].reverse() }),
  () =>
    reconstructNative({
      ...capture,
      code: capture.code.replace('sourceMappingURL', 'missingSourceMap'),
    }),
  () =>
    reconstructNative({
      ...capture,
      sourceMapURL: 'data:application/json;base64,e30=',
    }),
  () => decodeMap(capture.code, ''),
])(
  'refuses contradictory or unmapped real producer evidence instead of inferring identities',
  (action) => expect(action).toThrow(CoverageError),
)

it('refuses a stale native function total despite actual public code and ranges', () => {
  const source = parseLcov(result.value.lcov).find(
    (source) => source.file === 'source.ts',
  )
  if (!source) throw new CoverageError('missing native fixture record')
  expect(() =>
    validateNative(reconstructNative(capture), {
      ...source,
      foundFunctions: source.foundFunctions + 1,
    }),
  ).toThrow(CoverageError)
})

it('refuses named public functions when a corrupted copy of the actual anonymous profile supplies them', () => {
  const profile = capture.profile
  if (!profile) throw new CoverageError('missing actual fixture profile')
  expect(() =>
    parseProfiles([
      {
        ...profile,
        functions: profile.functions.map((fn) => ({
          ...fn,
          functionName: 'unsupported',
        })),
      },
    ]),
  ).toThrow(CoverageError)
})

it('refuses named LCOV data instead of assigning native anonymous functions an invented name', () => {
  const [named] = parseLcov(
    'SF:untrusted.ts\nFN:1,untrusted\nFNDA:1,untrusted\nFNF:1\nFNH:1\nDA:1,1\nLF:1\nLH:1\nend_of_record',
  )
  if (!named) throw new CoverageError('missing invalid observation')
  expect(() => validateNative(reconstructNative(capture), named)).toThrow(
    CoverageError,
  )
})

it('refuses contradictory native zero hits against the actual mapped block evidence', () => {
  const source = parseLcov(result.value.lcov).find(
    (source) => source.file === 'source.ts',
  )
  if (!source) throw new CoverageError('missing native fixture record')
  expect(() =>
    validateNative(reconstructNative(capture), {
      ...source,
      lines: new Map([...source.lines].map(([line]) => [line, 0])),
    }),
  ).toThrow(CoverageError)
})

it('retains native identity when the documented public profile omits a collected script', () => {
  const source = parseLcov(result.value.lcov).find(
    (source) => source.file === 'source.ts',
  )
  if (!source) throw new CoverageError('missing native fixture record')
  const view = reconstructNative({ ...capture, profile: undefined }, source)
  expect(() => validateNative(view, source)).not.toThrow()
  expect(view.functions.length).toBe(2)
})

it('rejects a missing public frame when no possible tail reproduces the native function inventory', () => {
  const source = parseLcov(result.value.lcov).find(
    (source) => source.file === 'source.ts',
  )
  if (!source) throw new CoverageError('missing native fixture record')
  expect(() =>
    reconstructNative(
      { ...capture, profile: undefined },
      { ...source, foundFunctions: source.foundFunctions + 10 },
    ),
  ).toThrow(CoverageError)
})

it('prevents native row removal from a captured receipt instead of trusting its class prototype', () => {
  const value = result.value
  const original = value.lcov
  try {
    expect(() =>
      Object.defineProperty(value, 'lcov', {
        value: original.replace(
          /TN:\nSF:source\.ts\n[\s\S]*?end_of_record\n/,
          '',
        ),
        configurable: true,
      }),
    ).toThrow()
  } finally {
    if (value.lcov !== original)
      Object.defineProperty(value, 'lcov', {
        value: original,
        configurable: true,
      })
  }
})

it('preserves native lines when a consumer mutates the exposed line collection', () => {
  const bound = VerifiedSourceCoverage.bind(result, root).find(
    (source) => source.file === 'source.ts',
  )
  if (!bound) throw new CoverageError('missing verified fixture')
  const expected = [...bound.lines]
  bound.lines.clear()
  expect([...bound.lines]).toEqual(expected)
})

it('rejects incomplete coverage per actual source even when its sibling source is fully covered', async () => {
  const local = mkdtempSync(join(tmpdir(), 'devup-producer-coarse-'))
  try {
    const config = join(local, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(local, 'source.ts'),
      'export function first() {\n  return 1\n}\nexport function second() {\n  return 2\n}\n',
    )
    writeFileSync(
      join(local, 'source.test.ts'),
      "import {it,expect} from 'bun:test';import {first} from './source';it('actual',()=>expect(first()).toBe(1))",
    )
    const partial = await captureProcess({
      root: local,
      config,
      coverage: join(local, 'coverage'),
      group: 'isolated',
      files: ['source.test.ts'],
      executable: process.execPath,
    })
    const sources = VerifiedSourceCoverage.bind(partial, local)
    const view = sources.find((source) => source.file === 'source.ts')
    expect(view?.hitFunctions).toBe(1)
    expect(view?.foundFunctions).toBe(2)
    expect(() =>
      auditCoverage(mergeCoverage([sources]), ['source.ts']),
    ).toThrow(CoverageError)
  } finally {
    rmSync(local, { recursive: true, force: true })
  }
}, 120000)
