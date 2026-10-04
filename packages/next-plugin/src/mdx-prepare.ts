import { readFile } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, isAbsolute, join, resolve } from 'node:path'
import { parse } from 'node:querystring'
import { fileURLToPath } from 'node:url'

import {
  createMdxOptionsInstance,
  type MdxOptionsInstance,
} from './mdx-options-instance'
import {
  isMdxRecord,
  type MdxLoader,
  type MdxPipeline,
  requireMdxPipeline,
} from './mdx-pipeline'
import { prepared, type PreparedMdx } from './mdx-prepare-result'
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
  ) {
    const position =
      isMdxRecord(cause) && isMdxRecord(cause.place) ? cause.place : {}
    const start = isMdxRecord(position.start) ? position.start : position
    const line = typeof start.line === 'number' ? start.line : 1
    const column = typeof start.column === 'number' ? start.column : 1
    super(
      `${filename}:${line}:${column}: devup-ui MDX preparation cannot use \`${filename}\` at build time: ${cause instanceof Error ? cause.message : String(cause)}; needs the configured compiler to complete successfully`,
      { cause },
    )
    this.line = line
    this.column = column
  }
}

type RunnerOptions = {
  readonly resource: string
  readonly loaders: readonly MdxLoader[]
  readonly context: {
    readonly rootContext: string
    readonly sourceMap?: boolean
    readonly mode?: string
    readonly _compiler?: object
    readonly devupMdxPrewarm: ReadonlyMap<number, MdxPrewarmStep>
    readonly getOptions: (this: { readonly query: unknown }) => unknown
    readonly emitError: (error: unknown) => void
  }
  readonly readResource: typeof readFile
}
type RunLoaders = (
  options: RunnerOptions,
  callback: (error: unknown, result: unknown) => void,
) => void

function isRunLoaders(value: unknown): value is RunLoaders {
  return typeof value === 'function'
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
  if (signal.aborted) throw new MdxCompileError(filename, signal.reason)
  if (timeoutMs <= 0)
    throw new MdxCompileError(filename, 'MDX preparation deadline exceeded')
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
    const loaders = instance.loadersFor(pipeline).map((loader, index) => {
      const options = loader.options
      const file = projectRequire.resolve(
        isAbsolute(loader.loader)
          ? loader.loader
          : loader.loader.startsWith('.')
            ? resolve(root, loader.loader)
            : loader.loader,
      )
      const step = recognizeMdxPrewarmStep(file)
      if (step)
        devupMdxPrewarm.set(index, {
          ...step,
          compiler: mdxPrewarmCacheOwner(context, pipeline, dirname(filename)),
        })
      return {
        ...loader,
        loader: step
          ? fileURLToPath(
              new URL(
                import.meta.url.endsWith('.ts')
                  ? './mdx-prewarm-loader.ts'
                  : './mdx-prewarm-loader.js',
                import.meta.url,
              ),
            )
          : file,
        ...(step ? { type: 'module' } : {}),
        ...(typeof options === 'string' || isMdxRecord(options)
          ? { options }
          : {}),
        ...(isMdxRecord(options)
          ? { ident: loader.ident ?? `devup-mdx-preparation-${index}` }
          : {}),
      }
    })
    return await new Promise<PreparedMdx>((resolveOutput, reject) => {
      let settled = false
      function finish(error: unknown, result?: unknown) {
        if (settled) return
        settled = true
        clearTimeout(timer)
        signal.removeEventListener('abort', abort)
        if (error) {
          reject(new MdxCompileError(filename, error))
          return
        }
        resolveOutput(Promise.resolve().then(() => prepared(filename, result)))
      }
      const abort = () => finish(signal.reason)
      const timer = setTimeout(() => finish('MDX compiler timeout'), timeoutMs)
      signal.addEventListener('abort', abort, { once: true })
      async function start() {
        if (settled) return
        runLoaders(
          {
            resource: filename,
            loaders,
            readResource: readFile,
            context: {
              rootContext: root,
              ...(context.sourceMap === undefined
                ? {}
                : { sourceMap: context.sourceMap }),
              ...(context.mode === undefined ? {} : { mode: context.mode }),
              ...(context.compiler === undefined
                ? {}
                : { _compiler: context.compiler }),
              devupMdxPrewarm,
              getOptions() {
                if (typeof this.query !== 'string') return this.query
                const query = this.query.replace(/^\?/, '')
                return query.startsWith('{') ? JSON.parse(query) : parse(query)
              },
              emitError: (error) => finish(error),
            },
          },
          finish,
        )
      }
      void start().catch(finish)
    })
  } catch (cause) {
    if (cause instanceof MdxCompileError) throw cause
    throw new MdxCompileError(filename, cause)
  }
}
