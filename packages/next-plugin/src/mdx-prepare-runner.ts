import { readFile } from 'node:fs'
import { parse } from 'node:querystring'

import type { MdxLoader } from './mdx-pipeline'
import { recordMdxDependencies } from './mdx-prepare-dependencies'
import { prepared, type PreparedMdx } from './mdx-prepare-result'
import type {
  MdxPreparationContext,
  MdxPrewarmStep,
} from './mdx-prewarm-boundary'

type RunnerOptions = {
  readonly resource: string
  readonly loaders: readonly MdxLoader[]
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
  readonly signal: AbortSignal
  readonly timeoutMs: number
  readonly loaders: readonly MdxLoader[]
  readonly context: MdxPreparationContext
  readonly steps: ReadonlyMap<number, MdxPrewarmStep>
  readonly runLoaders: RunLoaders
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
  return new Promise<PreparedMdx>((resolveOutput, reject) => {
    let settled = false
    function finish(error: unknown, result?: unknown) {
      if (settled) return
      settled = true
      clearTimeout(timer)
      signal.removeEventListener('abort', abort)
      if (error) {
        reject(error)
        return
      }
      resolveOutput(
        Promise.resolve().then(() => prepared(filename, result, dependencies)),
      )
    }
    const abort = () => finish(signal.reason)
    const timer = setTimeout(() => finish('MDX compiler timeout'), timeoutMs)
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
      devupMdxPrewarm: steps,
      getOptions(this: { readonly query: unknown }) {
        if (typeof this.query !== 'string') return this.query
        const query = this.query.replace(/^\?/, '')
        return query.startsWith('{') ? JSON.parse(query) : parse(query)
      },
      emitError: (error: unknown) => finish(error),
    }
    const dependencies = recordMdxDependencies(filename, loaderContext)
    async function start() {
      if (settled) return
      runLoaders(
        {
          resource: filename,
          loaders,
          readResource: readFile,
          context: loaderContext,
        },
        finish,
      )
    }
    void start().catch(finish)
  })
}
