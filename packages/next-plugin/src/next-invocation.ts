import { join, resolve } from 'node:path'

import type {
  NextInvocationDecision,
  NextInvocationInput,
} from './next-invocation-types'

export type {
  NextInvocationDecision,
  NextInvocationInput,
  NextInvocationRuntime,
} from './next-invocation-types'

const SUPPORTED_RANGE = '16.3.x'
const ADAPTER_RECEIVE = {
  kind: 'receive',
  authority: 'adapter',
  isolation: 'current-isolate',
} as const
const WEBPACK_RECEIVE = {
  kind: 'receive',
  authority: 'webpack-beforeCompile',
  isolation: 'current-isolate',
} as const
const STAGES = {
  wrapper: { kind: 'inert', reason: 'wrapper' },
  cache: { kind: 'inert', reason: 'cache' },
  'supplied-config': { kind: 'inert', reason: 'supplied-config' },
  'webpack-beforeCompile': WEBPACK_RECEIVE,
  adapter: undefined,
} as const satisfies Record<
  NextInvocationInput['stage'],
  NextInvocationDecision | undefined
>

export class NextInvocationError extends Error {
  readonly installedVersion: string
  readonly supportedRange = SUPPORTED_RANGE
  readonly configFile: string
  readonly reason: string

  constructor(input: NextInvocationInput, reason: string) {
    super(
      `${input.evaluation.configFile}:1:1: devup-ui MDX invocation cannot use ` +
        `\`${input.evaluation.phase}\` at build time: installed Next ${input.installed.version}; ` +
        `supported ${SUPPORTED_RANGE}; ${reason}; needs use the qualified Next CLI on ` +
        `${SUPPORTED_RANGE}, or webpack with its real beforeCompile gate`,
    )
    this.name = 'NextInvocationError'
    this.installedVersion = input.installed.version
    this.configFile = input.evaluation.configFile
    this.reason = reason
  }
}

export function classifyNextInvocation(
  input: NextInvocationInput,
): NextInvocationDecision {
  if (!input.required) return { kind: 'inert', reason: 'unneeded' }
  const stage = STAGES[input.stage]
  if (stage) return stage
  if (!/^16\.3\.(0|[1-9]\d*)$/.test(input.installed.version)) {
    throw new NextInvocationError(
      input,
      'unverified Next minor/release profile',
    )
  }
  const { evaluation, runtime, config } = input
  if (
    resolve(evaluation.projectDir) !== resolve(evaluation.expectedProjectDir)
  ) {
    throw new NextInvocationError(
      input,
      'adapter project differs from captured project',
    )
  }
  const root = resolve(input.installed.packageDir)
  const entry = runtime.entryPath ? resolve(runtime.entryPath) : undefined
  const installed = (path: string): string => join(root, path)
  const modules = new Set(runtime.loadedModules.map((path) => resolve(path)))
  if (entry === installed('dist/telemetry/detached-flush.js')) {
    return { kind: 'inert', reason: 'telemetry' }
  }
  if (
    evaluation.phase === 'phase-production-server' ||
    evaluation.phase === 'phase-export'
  ) {
    return { kind: 'inert', reason: 'non-compiling' }
  }
  if (runtime.env.NEXT_RSPACK) {
    throw new NextInvocationError(
      input,
      'Rspack receiving profile is unverified',
    )
  }
  const receiving = runtime.env.TURBOPACK ? ADAPTER_RECEIVE : WEBPACK_RECEIVE
  if (evaluation.phase === 'phase-development-server') {
    if (
      entry === installed('dist/server/lib/start-server.js') &&
      runtime.isMainThread &&
      runtime.hasIpc &&
      runtime.env.__NEXT_DEV_SERVER === '1' &&
      runtime.env.NEXT_PRIVATE_WORKER === '1'
    ) {
      return receiving
    }
    throw new NextInvocationError(
      input,
      'development invocation lacks stock CLI child proof',
    )
  }
  if (evaluation.phase !== 'phase-production-build') {
    throw new NextInvocationError(input, 'unknown compilation phase')
  }
  const thread = entry === installed('dist/compiled/jest-worker/threadChild.js')
  const child = entry === installed('dist/compiled/jest-worker/processChild.js')
  if (thread || child) {
    if (modules.has(installed('dist/build/worker.js'))) {
      return { kind: 'inert', reason: 'supplied-config' }
    }
    const turbo = modules.has(installed('dist/build/turbopack-build/impl.js'))
    const webpack = modules.has(installed('dist/build/webpack-build/impl.js'))
    if (
      runtime.env.NEXT_PRIVATE_BUILD_WORKER === '1' &&
      runtime.env.IS_NEXT_WORKER === 'true' &&
      turbo !== webpack &&
      ((thread && !runtime.isMainThread && turbo) ||
        (child && runtime.isMainThread && runtime.hasIpc && webpack)) &&
      input.build &&
      resolve(input.build.projectDir) === resolve(evaluation.projectDir)
    ) {
      return turbo ? ADAPTER_RECEIVE : WEBPACK_RECEIVE
    }
    throw new NextInvocationError(
      input,
      'Jest bootstrap/compilation target/marker proof is incomplete',
    )
  }
  if (
    entry !== installed('dist/bin/next') ||
    runtime.argv[2] !== 'build' ||
    !runtime.isMainThread ||
    !modules.has(installed('dist/cli/next-build.js')) ||
    !input.build ||
    resolve(input.build.projectDir) !== resolve(evaluation.projectDir)
  ) {
    throw new NextInvocationError(
      input,
      'build invocation lacks stock CLI/context proof',
    )
  }
  if (input.build.mode === 'generate' || input.build.mode === 'generate-env') {
    return { kind: 'inert', reason: 'non-compiling' }
  }
  if (
    input.build.mode !== undefined &&
    input.build.mode !== 'default' &&
    input.build.mode !== 'compile'
  ) {
    throw new NextInvocationError(input, 'unknown native build mode')
  }
  const worker = runtime.env.TURBOPACK
    ? runtime.env.NEXT_TURBOPACK_USE_WORKER !== '0'
    : (config.webpackBuildWorker ?? !config.hasWebpack)
  return worker
    ? {
        kind: 'handoff',
        target: runtime.env.TURBOPACK
          ? 'turbopack-build-thread'
          : 'webpack-build-process',
        prepared: false,
      }
    : receiving
}
