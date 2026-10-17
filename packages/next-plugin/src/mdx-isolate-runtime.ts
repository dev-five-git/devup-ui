import { createRequire } from 'node:module'
import { isAbsolute, join, resolve } from 'node:path'

import { mdxLoaderPath } from './mdx-invocation'
import {
  checkMdxIsolateScope,
  createMdxIsolateAuthority,
  currentMdxIsolateFailureScope,
} from './mdx-isolate-authority'
import { isMdxIsolateParserDiagnostic } from './mdx-isolate-diagnostic'
import { equalMdxIsolatePipeline } from './mdx-isolate-pipeline'
import {
  type MdxIsolateDemand,
  type MdxIsolateFailure,
  type MdxIsolateOutcome,
  type MdxIsolateVerdict,
  requireMdxIsolateData,
} from './mdx-isolate-protocol'
import { createMdxOptionsInstance } from './mdx-options-instance'
import { compileMdx, createMdxDeadline, MdxCompileError } from './mdx-prepare'
import { recognizeMdxPrewarmStep } from './mdx-prewarm-boundary'
import {
  captureMdxFreshness,
  changedMdxInput,
  createMdxTimestampAccuracy,
  fingerprintMdxInput,
  MdxFreshnessError,
} from './mdx-source-freshness'

export function createMdxIsolateRuntime(root: string) {
  const installed = createRequire(join(root, 'package.json'))
  const runs = new Map<
    string,
    {
      readonly options: ReturnType<typeof createMdxOptionsInstance>
      readonly pipelines: MdxIsolateDemand['pipeline'][]
      readonly jobs: Map<string, Promise<MdxIsolateOutcome>>
      readonly successes: Map<
        string,
        Extract<MdxIsolateOutcome, { readonly kind: 'prepared' }>
      >
    }
  >()
  let firstFailure:
    Extract<MdxIsolateOutcome, { readonly kind: 'failed' }> | undefined
  const retired = new Set<string>()
  function check(run: string, filename: string): MdxIsolateVerdict {
    try {
      if (retired.has(run)) throw new TypeError('run retired')
      const success = runs.get(run)?.successes.get(filename)
      if (!success) throw new TypeError('resource has no prepared native scope')
      checkMdxIsolateScope(success.output)
      return { run, filename }
    } catch (cause) {
      const error = new MdxCompileError(filename, cause)
      return {
        run,
        filename,
        error: {
          filename,
          message: error.message,
          line: error.line,
          column: error.column,
        },
      }
    }
  }
  function finalize() {
    return [...runs].flatMap(([run, state]) =>
      [...state.successes.keys()].map((filename) => check(run, filename)),
    )
  }
  function quarantine(error: MdxIsolateFailure) {
    firstFailure ??= { kind: 'failed', error }
    const terminal = finalize()
    firstFailure = { ...firstFailure, terminal }
    return terminal
  }
  const compile = async (
    request: MdxIsolateDemand,
  ): Promise<MdxIsolateOutcome> => {
    if (retired.has(request.run))
      return {
        kind: 'failed',
        recoverable: true,
        error: {
          filename: request.filename,
          line: 1,
          column: 1,
          message: 'MDX isolate run retired',
        },
      }
    if (firstFailure) return firstFailure
    let run = runs.get(request.run)
    if (!run) {
      run = {
        options: createMdxOptionsInstance(),
        pipelines: [],
        jobs: new Map(),
        successes: new Map(),
      }
      runs.set(request.run, run)
    }
    const previous = run.jobs.get(request.filename)
    if (previous) return previous
    let pipeline = run.pipelines.find((candidate) =>
      equalMdxIsolatePipeline(candidate, request.pipeline),
    )
    if (!pipeline) {
      pipeline = request.pipeline
      run.pipelines.push(pipeline)
    }
    const optionsInstance = run.options
    const selected = pipeline
    const state = run
    let physicalModule: string | undefined
    const job = Promise.resolve().then(async (): Promise<MdxIsolateOutcome> => {
      try {
        const loader = selected.loaders[0]
        if (!loader) throw new TypeError('missing isolated MDX compiler')
        const path = mdxLoaderPath(loader)
        const compiler = installed.resolve(
          isAbsolute(path)
            ? path
            : path.startsWith('.')
              ? resolve(root, path)
              : path,
        )
        const context = createMdxIsolateAuthority(root, compiler)
        const step = recognizeMdxPrewarmStep(compiler)
        if (!step) throw new TypeError('physical source recognition changed')
        physicalModule = step.module
        const original = fingerprintMdxInput(request.filename, {
          kind: 'file',
          path: request.filename,
          loader: 'loader-runner/resource',
        })
        const startedAt = Date.now()
        const output = await compileMdx({
          root,
          filename: request.filename,
          pipeline: selected,
          optionsInstance,
          context,
          signal: new AbortController().signal,
          deadline: createMdxDeadline(request.timeoutMs, request.timeoutMs),
        })
        const inputs = captureMdxFreshness(output, {
          startedAt,
          development: false,
          accuracy: createMdxTimestampAccuracy(),
        })
        const changed = changedMdxInput(request.filename, [original])
        if (changed)
          throw new MdxFreshnessError(
            request.filename,
            changed,
            'resource changed during provisional compilation',
          )
        const outcome = {
          kind: 'prepared',
          output,
          inputs,
          physical: [{ loader: compiler, module: step.module }],
        } as const
        requireMdxIsolateData(outcome)
        state.successes.set(request.filename, outcome)
        return outcome
      } catch (cause) {
        const error =
          cause instanceof MdxCompileError
            ? cause
            : new MdxCompileError(request.filename, cause, selected)
        const failure = {
          kind: 'failed',
          error: {
            filename: error.filename,
            message: error.message,
            line: error.line,
            column: error.column,
          },
        } as const
        if (
          physicalModule &&
          cause instanceof MdxCompileError &&
          (await isMdxIsolateParserDiagnostic(
            cause.cause,
            physicalModule,
            selected,
          )) &&
          currentMdxIsolateFailureScope(cause.cause)
        )
          return { ...failure, recoverable: true }
        const fatal = {
          ...failure.error,
          message: `${failure.error.message}; configured plugin/loader chain ${JSON.stringify(selected.loaders)}; isolate quarantined; no new publication; recover with a new build or dev-server restart`,
        }
        return { kind: 'failed', error: fatal, terminal: quarantine(fatal) }
      }
    })
    run.jobs.set(request.filename, job)
    return job
  }
  return Object.assign(compile, {
    check,
    quarantine,
    retire: (run: string) => {
      retired.add(run)
      runs.delete(run)
    },
  })
}
