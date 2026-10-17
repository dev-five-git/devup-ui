import { readFile } from 'node:fs'

import { createMdxContextAdmission } from './mdx-context-admission'
import type { MdxInvocationLoader } from './mdx-invocation'
import {
  isolatePreparationSteps,
  rememberMdxIsolateFailureScope,
  rememberMdxIsolateScope,
} from './mdx-isolate-authority'
import { rememberMdxPreparedScope } from './mdx-live-admission'
import {
  installMdxNormalCapture,
  type MdxNormalSeam,
} from './mdx-normal-capture'
import { installedMdxJsonParser } from './mdx-options-parser'
import { isMdxRecord } from './mdx-pipeline'
import { recordMdxDependencies } from './mdx-prepare-dependencies'
import { prepared, type PreparedMdx } from './mdx-prepare-result'
import type {
  MdxPreparationContext,
  MdxPrewarmStep,
} from './mdx-prewarm-boundary'

type RunnerOptions = {
  readonly resource: string
  readonly loaders: readonly MdxInvocationLoader[]
  readonly context: object
  readonly readResource: typeof readFile
}
export type RunLoaders = (
  options: RunnerOptions,
  callback: (error: unknown, result: unknown) => void,
) => void
export function isRunLoaders(value: unknown): value is RunLoaders {
  return typeof value === 'function'
}

export function runMdxLoaders(request: {
  readonly root: string
  readonly filename: string
  readonly resource?: string
  readonly signal: AbortSignal
  readonly timeoutMs: number
  readonly loaders: readonly MdxInvocationLoader[]
  readonly context: MdxPreparationContext
  readonly steps: ReadonlyMap<number, MdxPrewarmStep>
  readonly runLoaders: RunLoaders
  readonly seam?: MdxNormalSeam
}): Promise<PreparedMdx> {
  const {
    root,
    filename,
    signal,
    timeoutMs,
    loaders,
    context,
    steps,
    runLoaders,
  } = request
  const admission = createMdxContextAdmission(() =>
    installedMdxJsonParser(root),
  )
  return new Promise<PreparedMdx>((resolveOutput, reject) => {
    let settled = false
    let completing = false
    function finish(error: unknown, result?: unknown) {
      if (settled) return
      const failure = admission.close() ?? error
      settled = true
      clearTimeout(timer)
      signal.removeEventListener('abort', abort)
      if (failure) {
        rememberMdxIsolateFailureScope({
          context,
          failure,
          compilerPath: request.seam?.compilerPath,
          scope: admission.scope(),
        })
        reject(failure)
        return
      }
      resolveOutput(
        Promise.resolve().then(() => {
          admission.check()
          signal.throwIfAborted()
          if (isMdxRecord(result) && result.cacheable === false)
            throw new TypeError(
              'MDX preparation cannot reuse cacheable(false) output; use native compilation',
            )
          const output = prepared(filename, result, dependencies)
          rememberMdxPreparedScope(output, admission.scope())
          rememberMdxIsolateScope(context, output, admission.scope())
          return output
        }),
      )
    }
    const abort = () => finish(signal.reason)
    const timer = setTimeout(
      () => finish(capture?.timeoutError() ?? 'MDX compiler timeout'),
      timeoutMs,
    )
    signal.addEventListener('abort', abort, { once: true })
    const loaderContext = {
      rootContext: root,
      ...(context.sourceMap === undefined
        ? {}
        : { sourceMap: context.sourceMap }),
      ...(context.mode === undefined ? {} : { mode: context.mode }),
      ...(context.compiler === undefined
        ? {}
        : { _compiler: context.compiler }),
      emitError: (error: unknown) => finish(error),
    }
    const dependencies = recordMdxDependencies(filename, loaderContext)
    const capture = request.seam
      ? installMdxNormalCapture(loaderContext, request.seam, {
          admission,
          steps: isolatePreparationSteps({ root, context, steps }),
        })
      : undefined
    const callback = (error: unknown, result: unknown) => {
      if (settled || completing) return
      if (!capture) {
        finish(error, result)
        return
      }
      completing = true
      admission.close()
      void Promise.resolve()
        .then(() => capture.accept(error, result))
        .then((accepted) => finish(null, accepted), finish)
    }
    async function start() {
      if (settled) return
      runLoaders(
        {
          resource: request.resource ?? filename,
          loaders,
          readResource: readFile,
          context: loaderContext,
        },
        callback,
      )
    }
    void start().catch(finish)
  })
}
