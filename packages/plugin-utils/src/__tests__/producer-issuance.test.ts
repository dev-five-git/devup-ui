import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterAll, beforeAll, expect, it } from 'bun:test'

import { auditCoverage, mergeCoverage } from '../../../../test-harness/coverage'
import {
  CoverageError,
  parseLcov,
  type SourceCoverage,
} from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import type { NativeView } from '../../../../test-harness/producer-native'
import {
  captureProcess,
  ProducerResult,
} from '../../../../test-harness/producer-process'

const root = mkdtempSync(join(tmpdir(), 'devup-producer-issuance-'))
let partial: ProducerResult
let genuine: VerifiedSourceCoverage
let source: SourceCoverage
const invented: NativeView = {
  code: 'invented executed input',
  map: 'invented map',
  input: 'invented source input',
  functions: [
    { startOffset: 0, endOffset: 1, hasExecuted: true },
    { startOffset: 2, endOffset: 3, hasExecuted: true },
  ],
  lines: new Map([[1, 1]]),
  blockLines: new Set(),
  coarse: new Map(),
}
beforeAll(async () => {
  const config = join(root, 'bunfig.toml')
  writeFileSync(
    config,
    '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
  )
  writeFileSync(
    join(root, 'source.ts'),
    'export function first() { return 1 }; export function second() { return 2 }\n',
  )
  writeFileSync(
    join(root, 'partial.test.ts'),
    "import {it,expect} from 'bun:test';import {first} from './source';it('partial',()=>expect(first()).toBe(1))",
  )
  writeFileSync(
    join(root, 'complete.test.ts'),
    "import {it,expect} from 'bun:test';import {first,second} from './source';it('complete',()=>{expect(first()).toBe(1);expect(second()).toBe(2)})",
  )
  partial = await captureProcess({
    root,
    config,
    coverage: join(root, 'partial'),
    group: 'isolated',
    files: ['partial.test.ts'],
    executable: process.execPath,
  })
  const native = parseLcov(partial.value.lcov).find(
    (row) => row.file === 'source.ts',
  )
  if (!native) throw new CoverageError('missing partial issuance witness')
  source = native
  const complete = await captureProcess({
    root,
    config,
    coverage: join(root, 'complete'),
    group: 'isolated',
    files: ['complete.test.ts'],
    executable: process.execPath,
  })
  const verified = VerifiedSourceCoverage.bind(complete, root).find(
    (row) => row.file === 'source.ts',
  )
  if (!verified) throw new CoverageError('missing genuine issuance witness')
  genuine = verified
}, 120000)
afterAll(() => rmSync(root, { recursive: true, force: true }))

it.each(['native', 'exact-oracle'] as const)(
  'refuses reflected issuance when %s partial FNF2/FNH1 is paired with two invented executed functions',
  (kind) => {
    // Given: real partial native data, not an invented issuer receipt.
    expect(source.foundFunctions).toBe(2)
    expect(source.hitFunctions).toBe(1)
    expect([...source.lines.keys()]).toEqual([1])
    expect(source.lines.get(1)).toBeGreaterThan(0)
    const [exact] = parseLcov(
      'SF:a.ts\nFNF:2\nFNH:1\nDA:1,1\nLF:1\nLH:1\nend_of_record',
    )
    if (!exact) throw new CoverageError('missing exact oracle observation')
    const observation = kind === 'native' ? source : exact
    // When / Then: the original countercase cannot create a value for merge/audit.
    expect(() => {
      const forged: VerifiedSourceCoverage = Reflect.construct(
        VerifiedSourceCoverage,
        [observation, invented, partial.value.config],
      )
      auditCoverage(mergeCoverage([[forged]]), [observation.file])
    }).toThrow(CoverageError)
  },
)

it.each(['legacy', 'guessed'] as const)(
  'rejects %s reflected producer issuance before reading untrusted payload getters',
  (shape) => {
    // Given: a hostile value whose getters record whether the constructor touched it.
    let reads = 0
    const payload = {
      get captures() {
        reads++
        return []
      },
      get lcov() {
        reads++
        return partial.value.lcov
      },
    }
    const args = shape === 'legacy' ? [payload] : [Symbol('issuer'), payload]
    // When / Then: neither the old signature nor a guessed capability issues anything.
    expect(() => Reflect.construct(ProducerResult, args)).toThrow(CoverageError)
    expect(reads).toBe(0)
  },
)

it.each(['legacy', 'guessed'] as const)(
  'rejects %s reflected coverage issuance before reading untrusted source or view getters',
  (shape) => {
    // Given: getters must not run even when the attacker knows the new argument shape.
    let reads = 0
    const hostile = {
      get file() {
        reads++
        return 'source.ts'
      },
      get functions() {
        reads++
        return invented.functions
      },
      get source() {
        reads++
        return source
      },
    }
    const args =
      shape === 'legacy'
        ? [hostile, hostile, 'config']
        : [Symbol('issuer'), hostile]
    // When / Then: rejection is a provenance error, before payload evaluation.
    expect(() => Reflect.construct(VerifiedSourceCoverage, args)).toThrow(
      CoverageError,
    )
    expect(reads).toBe(0)
  },
)

it('refuses reflected producer data even when it copies every attribute of a genuine partial capture', () => {
  // Given: genuine data is public, issuance authority is not.
  const copied = { ...partial.value }
  // When / Then: replaying attributes cannot manufacture another producer lifetime.
  expect(() => {
    const forged: ProducerResult = Reflect.construct(ProducerResult, [copied])
    VerifiedSourceCoverage.bind(forged, root)
  }).toThrow(CoverageError)
})

it('refuses cross-class public payloads when prototypes and copied values are exchanged', () => {
  // Given: neither class can issue values belonging to the other authority.
  const producer: ProducerResult = Object.create(
    VerifiedSourceCoverage.prototype,
    { value: { value: partial.value } },
  )
  const coverage: SourceCoverage = Object.create(
    ProducerResult.prototype,
    Object.getOwnPropertyDescriptors(genuine),
  )
  // When / Then: copied genuine data carries no issuance capability or membership.
  expect(() => VerifiedSourceCoverage.bind(producer, root)).toThrow(
    CoverageError,
  )
  expect(() => mergeCoverage([[coverage]])).toThrow(CoverageError)
  expect(() => auditCoverage([coverage], ['source.ts'])).toThrow(CoverageError)
})

it.each(['merge', 'audit', 'peer', 'receiver'] as const)(
  'refuses own-attribute coverage prototype spoof at %s before payload getters',
  (boundary) => {
    // Given: own attributes override all prototype getters and pretend strict100.
    let reads = 0
    const spoof: VerifiedSourceCoverage = Object.create(
      VerifiedSourceCoverage.prototype,
      Object.fromEntries(
        [
          'file',
          'foundFunctions',
          'hitFunctions',
          'lines',
          'functions',
          'branches',
        ].map((key) => [
          key,
          {
            get() {
              reads++
              return key === 'file'
                ? 'source.ts'
                : key.endsWith('Functions')
                  ? 2
                  : new Map([[1, 1]])
            },
          },
        ]),
      ),
    )
    // When: each boundary is driven directly, including an invalid merge receiver.
    const actions = {
      merge: () => mergeCoverage([[spoof]]),
      audit: () => auditCoverage([spoof], ['source.ts']),
      peer: () => genuine.merge(spoof),
      receiver: () =>
        Reflect.apply(VerifiedSourceCoverage.prototype.merge, spoof, [genuine]),
    }
    // Then: refuse before trusting any public field or accessing private payload.
    expect(actions[boundary]).toThrow(CoverageError)
    expect(reads).toBe(0)
  },
)

it('accepts genuine capture and merge while keeping genuine partial native coverage incomplete', () => {
  // Given: both values originated from actual public producer captures.
  const bound = VerifiedSourceCoverage.bind(partial, root).find(
    (row) => row.file === 'source.ts',
  )
  if (!bound) throw new CoverageError('missing genuine partial source')
  // When / Then: genuine union remains accepted, partial alone remains below100.
  expect(() =>
    auditCoverage(mergeCoverage([[genuine], [genuine]]), ['source.ts']),
  ).not.toThrow()
  expect(() => auditCoverage(mergeCoverage([[bound]]), ['source.ts'])).toThrow(
    CoverageError,
  )
})
