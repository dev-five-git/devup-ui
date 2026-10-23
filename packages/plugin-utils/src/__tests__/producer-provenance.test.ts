import { expect, it } from 'bun:test'

import { auditCoverage, mergeCoverage } from '../../../../test-harness/coverage'
import {
  CoverageError,
  type SourceCoverage,
} from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { ProducerResult } from '../../../../test-harness/producer-process'

it.each(['prototype', 'foreign'] as const)(
  'refuses %s producer spoof at bind before its own value getter',
  (kind) => {
    // Given: unissued prototype/foreign data deliberately throws if its payload is read.
    let reads = 0
    class ForeignProducer {}
    const spoof: ProducerResult = Object.create(
      kind === 'prototype'
        ? ProducerResult.prototype
        : ForeignProducer.prototype,
      {
        value: {
          get() {
            reads++
            throw new TypeError('untrusted payload read')
          },
        },
      },
    )
    // When / Then: runtime membership, not prototype identity, controls admission.
    expect(() => VerifiedSourceCoverage.bind(spoof, '.')).toThrow(CoverageError)
    expect(reads).toBe(0)
  },
)

it('refuses producer value access when its genuine getter is applied to an unissued prototype', () => {
  // Given: a public getter cannot serve as an alternate receipt issuer.
  const getter = Object.getOwnPropertyDescriptor(
    ProducerResult.prototype,
    'value',
  )?.get
  if (!getter) throw new CoverageError('missing producer value getter')
  const spoof: ProducerResult = Object.create(ProducerResult.prototype)
  // When / Then: provenance rejection precedes private field access.
  expect(() => Reflect.apply(getter, spoof, [])).toThrow(CoverageError)
})

it.each(['merge', 'audit'] as const)(
  'refuses foreign coverage attributes at %s before their file getter',
  (boundary) => {
    // Given: structurally compatible foreign data is not an issued source.
    let reads = 0
    class ForeignCoverage {
      get file() {
        reads++
        return 'source.ts'
      }
    }
    const spoof: SourceCoverage = Object.assign(new ForeignCoverage(), {
      foundFunctions: 2,
      hitFunctions: 2,
      lines: new Map([[1, 1]]),
      functions: new Map(),
      branches: new Map(),
    })
    // When / Then: neither public boundary reads the untrusted file.
    expect(() =>
      boundary === 'merge'
        ? mergeCoverage([[spoof]])
        : auditCoverage([spoof], ['source.ts']),
    ).toThrow(CoverageError)
    expect(reads).toBe(0)
  },
)
