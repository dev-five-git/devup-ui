import { createRequire } from 'node:module'
import { dirname, isAbsolute, join, resolve } from 'node:path'

import {
  type MdxInvocation,
  MdxLoaderExecutionError,
  mdxLoaderPath,
  requireMdxInvocation,
} from './mdx-invocation'
import {
  createMdxOptionsInstance,
  type MdxOptionsInstance,
} from './mdx-options-instance'
import {
  isMdxRecord,
  type MdxPipeline,
  requireMdxPipeline,
} from './mdx-pipeline'
import type { PreparedMdx } from './mdx-prepare-result'
import { isRunLoaders, runMdxLoaders } from './mdx-prepare-runner'
import {
  type MdxPreparationContext,
  mdxPrewarmCacheOwner,
  type MdxPrewarmStep,
  recognizeMdxPrewarmStep,
} from './mdx-prewarm-boundary'

export {
  createMdxOptionsInstance,
  type MdxOptionsInstance,
} from './mdx-options-instance'
export type { PreparedMdx } from './mdx-prepare-result'
export type { MdxPreparationContext } from './mdx-prewarm-boundary'

export type MdxDeadline = {
  readonly expiresAt: number
  readonly individualMs: number
}
export type MdxCompileRequest = {
  readonly root: string
  readonly filename: string
  readonly pipeline: MdxPipeline
  readonly signal: AbortSignal
  readonly deadline: MdxDeadline
  readonly context?: MdxPreparationContext
  readonly optionsInstance?: MdxOptionsInstance
  readonly invocation?: MdxInvocation
}

// 30s preparation + 60s completion leaves 30s of the 120s client budget.
export function createMdxDeadline(
  totalMs = 30_000,
  individualMs = 10_000,
): MdxDeadline {
  return {
    expiresAt: Date.now() + Math.min(totalMs, 30_000),
    individualMs: Math.min(individualMs, 10_000),
  }
}

export class MdxCompileError extends Error {
  readonly name = 'MdxCompileError'
  readonly line: number
  readonly column: number
  constructor(
    readonly filename: string,
    cause: unknown,
    pipeline?: MdxPipeline,
  ) {
    const original =
      cause instanceof MdxLoaderExecutionError ? cause.cause : cause
    const position =
      isMdxRecord(original) && isMdxRecord(original.place) ? original.place : {}
    const start = isMdxRecord(position.start) ? position.start : position
    const line = typeof start.line === 'number' ? start.line : 1
    const column = typeof start.column === 'number' ? start.column : 1
    super(
      `${filename}:${line}:${column}: devup-ui MDX preparation cannot use \`${filename}\` at build time: ${cause instanceof Error ? cause.message : String(cause)}; rule "${pipeline?.ruleKey ?? '<unavailable>'}" loader ${pipeline?.loaders[0]?.loader ?? '<unavailable>'}; needs the configured compiler to complete successfully with observable original descriptors and representable loader context`,
      { cause: original },
    )
    this.line = line
    this.column = column
  }
}

export async function compileMdx(
  request: MdxCompileRequest,
): Promise<PreparedMdx> {
  const { root, filename, signal, deadline } = request
  const pipeline = requireMdxPipeline(filename, request.pipeline)
  const timeoutMs = Math.min(
    deadline.individualMs,
    10_000,
    deadline.expiresAt - Date.now(),
  )
  if (signal.aborted)
    throw new MdxCompileError(filename, signal.reason, pipeline)
  if (timeoutMs <= 0)
    throw new MdxCompileError(
      filename,
      'MDX preparation deadline exceeded',
      pipeline,
    )
  try {
    const projectRequire = createRequire(join(root, 'package.json'))
    const runner: unknown = projectRequire(
      'next/dist/compiled/loader-runner/LoaderRunner.js',
    )
    if (!isMdxRecord(runner) || !isRunLoaders(runner.runLoaders))
      throw new TypeError('project-installed Next loader runner is unavailable')
    const runLoaders = runner.runLoaders
    const devupMdxPrewarm = new Map<number, MdxPrewarmStep>()
    const context = request.context ?? { owner: {}, generation: {} }
    const instance = request.optionsInstance ?? createMdxOptionsInstance()
    const invocation =
      request.invocation &&
      requireMdxInvocation(pipeline, request.invocation, filename)
    const loaders = invocation
      ? instance.invocationLoadersFor(pipeline, invocation)
      : instance.loadersFor(pipeline).map((loader) => {
          const path = mdxLoaderPath(loader)
          const file = projectRequire.resolve(
            isAbsolute(path)
              ? path
              : path.startsWith('.')
                ? resolve(root, path)
                : path,
          )
          return { ...loader, loader: file + loader.loader.slice(path.length) }
        })
    const compilerIndex = invocation?.compilerIndex ?? 0
    const compiler = loaders[compilerIndex]
    if (!compiler) throw new TypeError('configured MDX compiler is unavailable')
    const compilerPath = mdxLoaderPath(compiler)
    const step = recognizeMdxPrewarmStep(compilerPath)
    if (step)
      devupMdxPrewarm.set(compilerIndex, {
        ...step,
        compiler: mdxPrewarmCacheOwner(context, pipeline, dirname(filename)),
      })
    return await runMdxLoaders({
      root,
      filename,
      ...(invocation ? { resource: invocation.resource } : {}),
      signal,
      timeoutMs,
      loaders,
      context,
      steps: devupMdxPrewarm,
      runLoaders,
      seam: {
        ruleKey: pipeline.ruleKey,
        compilerIndex,
        compilerPath,
        loaderCount: loaders.length,
        bridge: step !== undefined,
        ...(invocation ? { ownIndex: invocation.ownIndex } : {}),
      },
    })
  } catch (cause) {
    if (cause instanceof MdxCompileError) throw cause
    throw new MdxCompileError(filename, cause, pipeline)
  }
}
