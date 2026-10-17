import { expect, it } from 'bun:test'

import { classifyNextInvocation, NextInvocationError } from '../next-invocation'
import {
  installedPath,
  invocation,
  projectDir,
} from './next-invocation-fixture'

it.each(['16.3.0', '16.3.6', '16.3.99'])(
  'hands off Turbo on supported patch %s',
  (version) => {
    // Given: stock build bootstrap, final config and actual phase/project.
    const input = invocation()
    // When: the supported minor receives another patch.
    const result = classifyNextInvocation({
      ...input,
      installed: { ...input.installed, version },
    })
    // Then: no hash/patch gate or prepared parent.
    expect(result).toEqual({
      kind: 'handoff',
      target: 'turbopack-build-thread',
      prepared: false,
    })
  },
)

it.each(['16.2.9', '16.4.0', '17.0.0', '16.3.6-canary.1', 'broken'])(
  'locates required unsupported version %s',
  (version) => {
    // Given: MDX authority is required on an unverified minor/release.
    const input = invocation()
    // When / Then: fail with installed version, supported range, config and fix.
    expect(() =>
      classifyNextInvocation({
        ...input,
        installed: { ...input.installed, version },
      }),
    ).toThrow(NextInvocationError)
    expect(() =>
      classifyNextInvocation({
        ...input,
        installed: { ...input.installed, version },
      }),
    ).toThrow(`${input.evaluation.configFile}:1:1:`)
    expect(() =>
      classifyNextInvocation({
        ...input,
        installed: { ...input.installed, version },
      }),
    ).toThrow(`installed Next ${version}; supported 16.3.x`)
  },
)

it('receives Turbo in main when explicit worker override differs from default', () => {
  // Given: default worker ON; explicit override OFF.
  const input = invocation()
  // When: evaluate final adapter with actual environment override.
  const result = classifyNextInvocation({
    ...input,
    runtime: {
      ...input.runtime,
      env: { TURBOPACK: 'auto', NEXT_TURBOPACK_USE_WORKER: '0' },
    },
  })
  // Then: main, not an intent-only parent, receives.
  expect(result).toEqual({
    kind: 'receive',
    authority: 'adapter',
    isolation: 'current-isolate',
  })
})

it.each([
  [undefined, false, 'handoff'],
  [undefined, true, 'receive'],
  [true, true, 'handoff'],
  [false, false, 'receive'],
] as const)(
  'uses final webpack choice %s / callback %s',
  (webpackBuildWorker, hasWebpack, kind) => {
    // Given: explicit settings differ from the callback-dependent fallback.
    const input = invocation()
    // When: classify stock webpack build final config.
    const result = classifyNextInvocation({
      ...input,
      runtime: { ...input.runtime, env: {} },
      config: { webpackBuildWorker, hasWebpack },
    })
    // Then: receiving webpack is still gated by REAL beforeCompile, not this adapter.
    expect(result.kind).toBe(kind)
    expect(result).toEqual(
      kind === 'receive'
        ? {
            kind,
            authority: 'webpack-beforeCompile',
            isolation: 'current-isolate',
          }
        : { kind, target: 'webpack-build-process', prepared: false },
    )
  },
)

it.each(['generate', 'generate-env'])('leaves build mode %s inert', (mode) => {
  // Given: stock build has no compilation in this mode.
  const input = invocation()
  // When / Then: no receiving owner or prepared parent.
  expect(
    classifyNextInvocation({ ...input, build: { projectDir, mode } }),
  ).toEqual({ kind: 'inert', reason: 'non-compiling' })
})

it.each(['phase-production-server', 'phase-export', 'phase-production-build'])(
  'leaves non-MDX unknown programmatic %s unchanged',
  (phase) => {
    // Given: no lifecycle MDX, unknown version and arbitrary custom bootstrap.
    const input = invocation()
    // When: bypass role policy explicitly.
    const result = classifyNextInvocation({
      ...input,
      required: false,
      installed: { ...input.installed, version: '12.0.1' },
      evaluation: { ...input.evaluation, phase },
      runtime: { ...input.runtime, entryPath: '/custom/server.js' },
    })
    // Then: ordinary prior behavior is untouched.
    expect(result).toEqual({ kind: 'inert', reason: 'unneeded' })
  },
)

it.each(['wrapper', 'cache', 'supplied-config'] as const)(
  'never binds new closures at %s stage',
  (stage) => {
    // Given: raw phase-function diagnostic, cached render load or supplied export config.
    const input = invocation()
    // When / Then: these are not final adapter/compiler evaluations, even with MDX.
    expect(classifyNextInvocation({ ...input, stage })).toEqual({
      kind: 'inert',
      reason: stage,
    })
  },
)

it('uses native compiler authority for programmatic webpack without stock proof', () => {
  // Given: Main has a REAL beforeCompile/NMF invocation, not a synthetic adapter flag.
  const input = invocation()
  // When / Then: native gate is independently authoritative; no CLI/minor floor there.
  expect(
    classifyNextInvocation({
      ...input,
      stage: 'webpack-beforeCompile',
      installed: { ...input.installed, version: '15.1.0' },
      runtime: { ...input.runtime, entryPath: '/custom/server.js' },
    }),
  ).toEqual({
    kind: 'receive',
    authority: 'webpack-beforeCompile',
    isolation: 'current-isolate',
  })
})

it.each(['phase-production-server', 'phase-export'])(
  'serves artifacts without binding in %s',
  (phase) => {
    // Given: Next serve/export does not compile.
    const input = invocation()
    // When / Then: adapter remains inert.
    expect(
      classifyNextInvocation({
        ...input,
        evaluation: { ...input.evaluation, phase },
      }),
    ).toEqual({ kind: 'inert', reason: 'non-compiling' })
  },
)

it.each([
  { entryPath: '/custom/server.js' },
  { entryPath: undefined },
  { isMainThread: false },
  { loadedModules: [] },
  { argv: ['node', installedPath('dist/bin/next'), 'dev'] },
  { env: { NEXT_RSPACK: '1' } },
] as const)('rejects required unproven main bootstrap %#', (override) => {
  // Given: phase/NODE_ENV/private flags cannot establish a stock invocation.
  const input = invocation()
  // When / Then: required unknown invocation has actionable config error.
  expect(() =>
    classifyNextInvocation({
      ...input,
      runtime: { ...input.runtime, ...override },
    }),
  ).toThrow(
    'use the qualified Next CLI on 16.3.x, or webpack with its real beforeCompile gate',
  )
})

it.each([
  undefined,
  { projectDir: '/other', mode: 'default' },
  { projectDir, mode: 'unknown' },
])('rejects unproven native build context %#', (build) => {
  // Given: missing, cross-project or unknown mode facts.
  const input = invocation()
  // When / Then: never infer compilation from phase alone.
  expect(() => classifyNextInvocation({ ...input, build })).toThrow(
    NextInvocationError,
  )
})

it('rejects cross-project adapter facts before receiving', () => {
  // Given: a reentrant load for another project.
  const input = invocation()
  // When / Then: no cross-project owner decision.
  expect(() =>
    classifyNextInvocation({
      ...input,
      evaluation: { ...input.evaluation, projectDir: '/other' },
    }),
  ).toThrow(NextInvocationError)
})

it('rejects unknown phase instead of assuming development', () => {
  // Given: required authority under an unknown phase.
  const input = invocation()
  // When / Then: fail closed.
  expect(() =>
    classifyNextInvocation({
      ...input,
      evaluation: { ...input.evaluation, phase: 'unknown' },
    }),
  ).toThrow(NextInvocationError)
})
