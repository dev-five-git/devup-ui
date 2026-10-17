import { expect, it } from 'bun:test'

import { classifyNextInvocation, NextInvocationError } from '../next-invocation'
import { installedPath, invocation } from './next-invocation-fixture'

it.each([true, false])(
  'receives dev child with Turbo=%s, including restart',
  (turbo) => {
    // Given: Next-forked start-server, private markers, IPC and actual dev phase.
    const input = invocation()
    // When: classify the current child; restart has these same facts in a NEW isolate.
    const result = classifyNextInvocation({
      ...input,
      evaluation: { ...input.evaluation, phase: 'phase-development-server' },
      runtime: {
        ...input.runtime,
        entryPath: installedPath('dist/server/lib/start-server.js'),
        hasIpc: true,
        env: {
          __NEXT_DEV_SERVER: '1',
          NEXT_PRIVATE_WORKER: '1',
          ...(turbo ? { TURBOPACK: 'auto' } : {}),
        },
      },
    })
    // Then: no predecessor owner or PID is part of the result.
    expect(result).toEqual({
      kind: 'receive',
      authority: turbo ? 'adapter' : 'webpack-beforeCompile',
      isolation: 'current-isolate',
    })
  },
)

it.each([
  { env: { __NEXT_DEV_SERVER: '1' }, hasIpc: true, isMainThread: true },
  { env: { NEXT_PRIVATE_WORKER: '1' }, hasIpc: true, isMainThread: true },
  {
    env: { __NEXT_DEV_SERVER: '1', NEXT_PRIVATE_WORKER: '1' },
    hasIpc: false,
    isMainThread: true,
  },
  {
    env: { __NEXT_DEV_SERVER: '1', NEXT_PRIVATE_WORKER: '1' },
    hasIpc: true,
    isMainThread: false,
  },
] as const)('rejects incomplete dev provenance %#', (runtime) => {
  // Given: exact entrypoint but missing Next producer facts.
  const input = invocation()
  // When / Then: phase or private markers alone cannot receive.
  expect(() =>
    classifyNextInvocation({
      ...input,
      evaluation: { ...input.evaluation, phase: 'phase-development-server' },
      runtime: {
        ...input.runtime,
        ...runtime,
        entryPath: installedPath('dist/server/lib/start-server.js'),
      },
    }),
  ).toThrow(NextInvocationError)
})

it('leaves telemetry inert despite inherited dev flags', () => {
  // Given: same phase/env as dev child, DIFFERENT installed bootstrap.
  const input = invocation()
  // When / Then: telemetry cannot prepare or rebind.
  expect(
    classifyNextInvocation({
      ...input,
      evaluation: { ...input.evaluation, phase: 'phase-development-server' },
      runtime: {
        ...input.runtime,
        entryPath: installedPath('dist/telemetry/detached-flush.js'),
        env: {
          __NEXT_DEV_SERVER: '1',
          NEXT_PRIVATE_WORKER: '1',
          TURBOPACK: 'auto',
        },
      },
    }),
  ).toEqual({ kind: 'inert', reason: 'telemetry' })
})

it.each([
  ['threadChild.js', false, 'turbopack-build', 'adapter'],
  ['processChild.js', true, 'webpack-build', 'webpack-beforeCompile'],
] as const)(
  'receives exact native Jest %s compilation isolate',
  (bootstrap, isMainThread, impl, authority) => {
    // Given: actual Jest bootstrap and IPC-required implementation loaded in this isolate.
    const input = invocation()
    // When: compilation marker producers match the loaded target.
    const result = classifyNextInvocation({
      ...input,
      runtime: {
        ...input.runtime,
        entryPath: installedPath(`dist/compiled/jest-worker/${bootstrap}`),
        isMainThread,
        hasIpc: isMainThread,
        env: { NEXT_PRIVATE_BUILD_WORKER: '1', IS_NEXT_WORKER: 'true' },
        loadedModules: [installedPath(`dist/build/${impl}/impl.js`)],
      },
    })
    // Then: native worker receives, never parent-global/PID reuse.
    expect(result).toEqual({
      kind: 'receive',
      authority,
      isolation: 'current-isolate',
    })
  },
)

it.each(['threadChild.js', 'processChild.js'])(
  'leaves static/export %s inert even with inherited compile marker',
  (bootstrap) => {
    // Given: static worker bootstrap lacks compilation target identity.
    const input = invocation()
    // When / Then: supplied nextConfig does not become compilation authority.
    expect(
      classifyNextInvocation({
        ...input,
        runtime: {
          ...input.runtime,
          entryPath: installedPath(`dist/compiled/jest-worker/${bootstrap}`),
          isMainThread: bootstrap === 'processChild.js',
          hasIpc: true,
          env: { NEXT_PRIVATE_BUILD_WORKER: '1', IS_NEXT_WORKER: 'true' },
          loadedModules: [installedPath('dist/build/worker.js')],
        },
      }),
    ).toEqual({ kind: 'inert', reason: 'supplied-config' })
  },
)

it.each([
  [
    'threadChild.js',
    true,
    true,
    { NEXT_PRIVATE_BUILD_WORKER: '1', IS_NEXT_WORKER: 'true' },
    ['dist/build/turbopack-build/impl.js'],
  ],
  [
    'processChild.js',
    true,
    false,
    { NEXT_PRIVATE_BUILD_WORKER: '1', IS_NEXT_WORKER: 'true' },
    ['dist/build/webpack-build/impl.js'],
  ],
  [
    'threadChild.js',
    false,
    false,
    { IS_NEXT_WORKER: 'true' },
    ['dist/build/turbopack-build/impl.js'],
  ],
  [
    'threadChild.js',
    false,
    false,
    { NEXT_PRIVATE_BUILD_WORKER: '1' },
    ['dist/build/turbopack-build/impl.js'],
  ],
  [
    'threadChild.js',
    false,
    false,
    { NEXT_PRIVATE_BUILD_WORKER: '1', IS_NEXT_WORKER: 'true' },
    [],
  ],
  [
    'threadChild.js',
    false,
    false,
    { NEXT_PRIVATE_BUILD_WORKER: '1', IS_NEXT_WORKER: 'true' },
    ['dist/build/webpack-build/impl.js'],
  ],
  [
    'processChild.js',
    true,
    true,
    { NEXT_PRIVATE_BUILD_WORKER: '1', IS_NEXT_WORKER: 'true' },
    ['dist/build/turbopack-build/impl.js'],
  ],
  [
    'threadChild.js',
    false,
    false,
    { NEXT_PRIVATE_BUILD_WORKER: '1', IS_NEXT_WORKER: 'true' },
    ['dist/build/turbopack-build/impl.js', 'dist/build/webpack-build/impl.js'],
  ],
] as const)(
  'rejects inconsistent Jest compilation proof %#',
  (bootstrap, isMainThread, hasIpc, env, targets) => {
    // Given: marker-only/target-only/wrong-isolate facts cannot certify a receiving worker.
    const input = invocation()
    // When / Then: unknown required worker invocation fails located.
    expect(() =>
      classifyNextInvocation({
        ...input,
        runtime: {
          ...input.runtime,
          entryPath: installedPath(`dist/compiled/jest-worker/${bootstrap}`),
          isMainThread,
          hasIpc,
          env,
          loadedModules: targets.map(installedPath),
        },
      }),
    ).toThrow(NextInvocationError)
  },
)

it('rejects programmatic dev even when Next sets __NEXT_DEV_SERVER', () => {
  // Given: NextCustomServer.prepare sets the dev flag but not stock fork identity.
  const input = invocation()
  // When / Then: getRequestHandler cannot guess a receiving Turbo config evaluation.
  expect(() =>
    classifyNextInvocation({
      ...input,
      evaluation: { ...input.evaluation, phase: 'phase-development-server' },
      runtime: {
        ...input.runtime,
        entryPath: '/custom/server.js',
        env: { __NEXT_DEV_SERVER: '1', TURBOPACK: 'auto' },
      },
    }),
  ).toThrow(NextInvocationError)
})
