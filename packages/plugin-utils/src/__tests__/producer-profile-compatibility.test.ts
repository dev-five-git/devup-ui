import { expect, it } from 'bun:test'

import { CoverageError } from '../../../../test-harness/lcov'
import {
  parseProducerIdentity,
  qualifyProducerProfiles,
} from '../../../../test-harness/producer-data'

const range = { startOffset: 0, endOffset: 32, count: 1 } as const
const fn = { functionName: '', isBlockCoverage: true, ranges: [range] } as const
const profiles = [
  {
    scriptId: '1',
    url: 'source.ts',
    functions: [fn],
  },
] as const

it('admits compatible profiles when identity belongs to the actual current Bun', () => {
  // Given: actual runtime metadata, never a substituted revision hash.
  const identity = {
    pid: process.pid,
    config: '[test]',
    bun: Bun.version,
    revision: Bun.revision,
    profiles,
  }
  // When: the existing admission boundary sees the current runtime.
  const result = parseProducerIdentity(identity, identity)
  // Then: admission retains the exact identity and parsed anonymous ranges.
  expect(result).toEqual({
    ...identity,
    profiles: profiles.map((script) => ({
      ...script,
      functions: script.functions.map((fn) => ({
        isBlockCoverage: fn.isBlockCoverage,
        ranges: fn.ranges,
      })),
    })),
  })
})

it.each([
  Bun.version,
  '1.4.999',
  '1.3.9',
  '1.5.0',
  '2.4.0',
  `${Bun.version}-canary`,
  '1.4.03',
])(
  'admits compatible profile data independently of Bun version %s',
  (version) => {
    // Given / When: schema qualification is independent of version policy.
    const result = qualifyProducerProfiles(version, profiles)
    // Then: exact parsed ranges remain available for strict native validation.
    expect(result).toEqual([
      {
        scriptId: '1',
        url: 'source.ts',
        functions: [{ isBlockCoverage: true, ranges: [range] }],
      },
    ])
  },
)

it.each([
  null,
  [{}],
  ...[
    { ...fn, functionName: 'named' },
    { ...fn, isBlockCoverage: 1 },
    ...[-1, 0.5, Number.MAX_SAFE_INTEGER + 1].flatMap((invalid) =>
      ['startOffset', 'endOffset', 'count'].map((field) => ({
        ...fn,
        ranges: [{ ...range, [field]: invalid }],
      })),
    ),
  ].map((invalid) => [{ ...profiles[0], functions: [invalid] }]),
])(
  'rejects malformed profiles with version context and the original typed cause',
  (profile) => {
    // Given: the current version must satisfy the unchanged strict parser.
    // When: malformed profile data crosses the qualification boundary.
    const qualify = () => qualifyProducerProfiles(Bun.version, profile)
    // Then: contextual diagnostics retain the underlying CoverageError.
    expect(qualify).toThrow(CoverageError)
    expect(qualify).toThrow(
      `Bun ${Bun.version}; qualified anonymous precise profile format`,
    )
    try {
      qualify()
      expect.unreachable('expected malformed profile to throw')
    } catch (error) {
      if (!(error instanceof CoverageError)) throw error
      expect(error.cause).toBeInstanceOf(CoverageError)
    }
  },
)

it('rethrows the exact original error when an untrusted profile getter throws', () => {
  // Given: a real accessor fails while the strict parser reads a profile.
  const original = new Error('hostile profile getter')
  const profile = {
    get scriptId() {
      throw original
    },
  }
  // When: qualification reaches that untrusted property.
  const qualify = () => qualifyProducerProfiles(Bun.version, [profile])
  // Then: the non-CoverageError is neither wrapped nor swallowed.
  try {
    qualify()
    expect.unreachable('expected hostile getter to throw')
  } catch (error) {
    expect(error).toBe(original)
  }
})
