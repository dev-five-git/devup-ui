import { createRequire } from 'node:module'
import { Worker } from 'node:worker_threads'

import { equalMdxIsolatePipeline } from './mdx-isolate-pipeline'
import {
  type MdxIsolateControl,
  type MdxIsolateDemand,
  MdxIsolateError,
  type MdxIsolateOutcome,
  type MdxIsolateVerdict,
  parseMdxIsolateDemand,
  parseMdxIsolateOutcome,
  parseMdxIsolateVerdict,
  requireMdxIsolateData,
} from './mdx-isolate-protocol'
import { requireMdxSafeCompilerOptions } from './mdx-native-scope'
import { createMdxOptionsInstance } from './mdx-options-instance'
import type { PreparedMdx } from './mdx-prepare-result'
import { recognizeMdxPrewarmStep } from './mdx-prewarm-boundary'
import { changedMdxInput, MdxFreshnessError } from './mdx-source-freshness'

function located(filename: string, cause: unknown) {
  return new MdxIsolateError({
    filename,
    line: 1,
    column: 1,
    message: `${filename}:1:1: dormant MDX isolate cannot prepare this resource: ${cause instanceof Error ? cause.message : String(cause)}`,
  })
}

export type MdxIsolateChannel = {
  readonly on: (
    event: 'message' | 'error' | 'exit',
    listener: (value: unknown) => void,
  ) => unknown
  readonly once: (
    event: 'message' | 'error' | 'exit',
    listener: (value: unknown) => void,
  ) => unknown
  readonly off: (
    event: 'message' | 'error' | 'exit',
    listener: (value: unknown) => void,
  ) => unknown
  readonly postMessage: (value: unknown) => void
  readonly terminate: () => Promise<number>
}

/** Dormant boundary: no canonical publication/admission/watch capability is accepted. */
export function createMdxIsolateOwner(
  root: string,
  spawn: (root: string) => MdxIsolateChannel = (project) =>
    new Worker(
      createRequire(import.meta.url).resolve(
        '@devup-ui/next-plugin/mdx-isolate-worker',
      ),
      { workerData: project },
    ),
) {
  let worker: MdxIsolateChannel | undefined
  let exited: Promise<void> | undefined
  let closing: Promise<void> | undefined
  let fatal: MdxIsolateError | undefined
  let disposed = false
  let tail = Promise.resolve()
  const retired = new Set<string>()
  const terminals = new Map<string, MdxIsolateVerdict>()
  const retained = new Map<
    string,
    {
      readonly pipeline: MdxIsolateDemand['pipeline']
      readonly outcome: Promise<MdxIsolateOutcome>
    }
  >()
  const key = (request: Pick<MdxIsolateDemand, 'run' | 'filename'>) =>
    JSON.stringify([request.run, request.filename])
  async function close() {
    if (closing) return closing
    if (!worker) return
    const owned = worker
    closing = Promise.resolve().then(async () => {
      await owned.terminate()
      await exited
    })
    return closing
  }
  function start() {
    if (worker) return worker
    worker = spawn(root)
    const owned = worker
    exited = new Promise<void>((resolve) => {
      owned.once('exit', () => {
        fatal ??= located(
          root,
          'isolate crash: owning worker exited; recover with a new build or dev-server restart',
        )
        resolve()
      })
    })
    owned.on('error', (cause: unknown) => {
      fatal ??= located(root, cause)
    })
    return owned
  }
  function enqueue<T>(work: () => Promise<T>): Promise<T> {
    const pending = tail.then(work)
    tail = pending.then(
      () => undefined,
      () => undefined,
    )
    return pending
  }
  function requireOpen(run: string, filename: string) {
    if (disposed) throw located(filename, 'owner disposed')
    if (retired.has(run)) throw located(filename, 'run retired')
  }
  function unavailable(filename: string) {
    return located(
      filename,
      `isolate crash, timeout or quarantine without a final resource verdict (${fatal?.message ?? 'unavailable'}); no new publication; recover with a new build or dev-server restart`,
    )
  }
  async function exchange<T>(
    request: MdxIsolateDemand | MdxIsolateControl,
    signal: AbortSignal,
    parse: (value: unknown) => T,
  ): Promise<T> {
    signal.throwIfAborted()
    const owned = start()
    return new Promise<T>((resolve, reject) => {
      let settled = false
      const finish = (cause: unknown, outcome?: { readonly value: T }) => {
        if (settled) return
        settled = true
        clearTimeout(timer)
        owned.off('message', message)
        owned.off('error', error)
        owned.off('exit', exit)
        signal.removeEventListener('abort', abort)
        if (outcome) resolve(outcome.value)
        else reject(cause)
      }
      const message = (value: unknown) => {
        try {
          finish(undefined, { value: parse(value) })
        } catch (cause) {
          finish(cause)
        }
      }
      const error = (cause: unknown) => finish(cause)
      const exit = () => finish(fatal ?? unavailable(request.filename))
      const abort = () => finish(signal.reason)
      const timer = setTimeout(
        () =>
          finish(
            located(
              request.filename,
              'isolate timeout; recover with a new build or dev-server restart',
            ),
          ),
        request.timeoutMs,
      )
      owned.once('message', message)
      owned.once('error', error)
      owned.once('exit', exit)
      signal.addEventListener('abort', abort, { once: true })
      try {
        owned.postMessage(request)
      } catch (cause) {
        finish(cause)
      }
    })
  }
  async function seal(request: Pick<MdxIsolateDemand, 'run' | 'filename'>) {
    if (!worker || closing) return
    const verdict = await exchange(
      { operation: 'finalize', ...request, timeoutMs: 5000 },
      new AbortController().signal,
      parseMdxIsolateVerdict,
    )
    if (verdict.run !== request.run || verdict.filename !== request.filename)
      throw located(request.filename, 'terminal acknowledgement mismatch')
    for (const terminal of verdict.terminal ?? []) {
      if (!retained.has(key(terminal)))
        throw located(request.filename, 'unknown terminal resource')
      terminals.set(key(terminal), terminal)
    }
  }
  async function execute(
    request: MdxIsolateDemand,
    signal: AbortSignal,
  ): Promise<MdxIsolateOutcome> {
    try {
      if (fatal)
        return { kind: 'failed', error: unavailable(request.filename).failure }
      if (disposed) throw located(request.filename, 'owner disposed')
      signal.throwIfAborted()
      const result = await exchange(request, signal, parseMdxIsolateOutcome)
      switch (result.kind) {
        case 'failed':
          if (result.error.filename !== request.filename)
            throw located(
              request.filename,
              'worker returned a different resource',
            )
          if (result.recoverable) return result
          for (const verdict of result.terminal ?? []) {
            if (!retained.has(key(verdict)))
              throw located(request.filename, 'unknown terminal resource')
            terminals.set(key(verdict), verdict)
          }
          throw new MdxIsolateError(result.error)
        case 'prepared':
          if (result.output.filename !== request.filename)
            throw located(
              request.filename,
              'worker returned a different resource',
            )
          return result
        default:
          return result satisfies never
      }
    } catch (cause) {
      fatal ??=
        cause instanceof MdxIsolateError
          ? cause
          : located(request.filename, cause)
      try {
        await close()
      } catch (cleanup) {
        fatal = new MdxIsolateError(fatal.failure, cleanup)
      }
      return { kind: 'failed', error: fatal.failure }
    }
  }
  return {
    prepare(input: MdxIsolateDemand, signal: AbortSignal): Promise<void> {
      if (retired.has(input.run)) return Promise.resolve()
      try {
        const request = parseMdxIsolateDemand(input)
        requireMdxIsolateData(request)
        for (const loader of request.pipeline.loaders)
          requireMdxSafeCompilerOptions(loader.options, 'wire')
        createMdxOptionsInstance().loadersFor(request.pipeline)
        const clone = structuredClone(request)
        const previous = retained.get(key(request))
        if (previous) {
          if (!equalMdxIsolatePipeline(previous.pipeline, clone.pipeline))
            throw located(
              request.filename,
              'different pipeline for retained run/resource',
            )
          return previous.outcome.then(() => undefined)
        }
        const outcome = enqueue(() => execute(clone, signal))
        retained.set(key(request), { pipeline: clone.pipeline, outcome })
        return outcome.then(() => undefined)
      } catch (cause) {
        fatal ??=
          cause instanceof MdxIsolateError
            ? cause
            : located(input.filename, cause)
        const failure: MdxIsolateOutcome = {
          kind: 'failed',
          error: fatal.failure,
        }
        const first = fatal
        retained.set(key(input), {
          pipeline: input.pipeline,
          outcome: Promise.resolve(failure),
        })
        return enqueue(async () => {
          try {
            await seal({ run: input.run, filename: input.filename })
          } catch (cause) {
            fatal ??= located(input.filename, cause)
          }
          try {
            await close()
          } catch (cleanup) {
            fatal = new MdxIsolateError(first.failure, cleanup)
          }
        })
      }
    },
    async bind(request: {
      readonly run: string
      readonly filename: string
      readonly canonical: PreparedMdx
    }): Promise<PreparedMdx> {
      requireOpen(request.run, request.filename)
      const retainedOutcome = retained.get(key(request))
      if (!retainedOutcome)
        throw fatal
          ? unavailable(request.filename)
          : located(request.filename, 'resource was not prepared')
      const outcome = await retainedOutcome.outcome
      requireOpen(request.run, request.filename)
      switch (outcome.kind) {
        case 'failed':
          throw new MdxIsolateError(outcome.error)
        case 'prepared':
          break
        default:
          return outcome satisfies never
      }
      const bound = await enqueue(async () => {
        requireOpen(request.run, request.filename)
        let verdict: MdxIsolateVerdict | undefined
        if (fatal) {
          await close()
          requireOpen(request.run, request.filename)
          verdict = terminals.get(key(request))
          if (!verdict) throw unavailable(request.filename)
        } else {
          try {
            verdict = await exchange(
              {
                operation: 'check',
                run: request.run,
                filename: request.filename,
                timeoutMs: 5000,
              },
              new AbortController().signal,
              (value) => {
                const checked = parseMdxIsolateVerdict(value)
                if (
                  checked.run !== request.run ||
                  checked.filename !== request.filename
                )
                  throw located(
                    request.filename,
                    'worker returned a different scope verdict',
                  )
                for (const terminal of checked.terminal ?? [])
                  if (!retained.has(key(terminal)))
                    throw located(request.filename, 'unknown terminal resource')
                return checked
              },
            )
          } catch (cause) {
            fatal ??= located(request.filename, cause)
            try {
              await close()
            } catch (cleanup) {
              throw new MdxIsolateError(fatal.failure, cleanup)
            }
            requireOpen(request.run, request.filename)
            throw unavailable(request.filename)
          }
        }
        if (verdict.error) {
          for (const terminal of verdict.terminal ?? []) {
            terminals.set(key(terminal), terminal)
          }
          fatal ??= new MdxIsolateError(verdict.error)
          try {
            await close()
          } catch (cleanup) {
            throw new MdxIsolateError(verdict.error, cleanup)
          }
          requireOpen(request.run, request.filename)
          throw new MdxIsolateError(verdict.error)
        }
        requireOpen(request.run, request.filename)
        if (
          request.canonical.filename !== request.filename ||
          outcome.output.source !== request.canonical.source
        )
          throw located(request.filename, 'true/false compiler bytes disagree')
        const changed = changedMdxInput(request.filename, outcome.inputs)
        if (changed)
          throw new MdxFreshnessError(
            request.filename,
            changed,
            'provisional input changed before resource binding',
          )
        for (const physical of outcome.physical ?? [])
          if (
            recognizeMdxPrewarmStep(physical.loader)?.module !== physical.module
          ) {
            const first = located(
              request.filename,
              'physical compiler source recognition changed',
            )
            fatal ??= first
            const cleanup: unknown[] = []
            try {
              await seal({ run: request.run, filename: request.filename })
            } catch (cause) {
              cleanup.push(cause)
            }
            try {
              await close()
            } catch (cause) {
              cleanup.push(cause)
            }
            const refusal = cleanup.length
              ? new MdxIsolateError(
                  first.failure,
                  new AggregateError(
                    cleanup,
                    'physical-source refusal cleanup failed',
                  ),
                )
              : first
            if (fatal === first) fatal = refusal
            throw refusal
          }
        return outcome.output
      })
      requireOpen(request.run, request.filename)
      return bound
    },
    retire(run: string): Promise<void> {
      if (retired.has(run)) return tail
      retired.add(run)
      return enqueue(async () => {
        try {
          if (worker && !fatal && !disposed) {
            const verdict = await exchange(
              { operation: 'retire', run, filename: root, timeoutMs: 5000 },
              new AbortController().signal,
              parseMdxIsolateVerdict,
            )
            if (
              verdict.run !== run ||
              verdict.filename !== root ||
              verdict.error
            )
              throw located(root, 'worker retirement acknowledgement mismatch')
          } else if (closing) await closing
        } catch (cause) {
          fatal ??= located(root, cause)
          await close()
          throw fatal
        } finally {
          for (const [id] of retained)
            if (JSON.parse(id)[0] === run) {
              retained.delete(id)
              terminals.delete(id)
            }
        }
      })
    },
    async dispose() {
      disposed = true
      try {
        await close()
      } catch (cause) {
        throw fatal ?? located(root, cause)
      }
      await tail
      retained.clear()
      terminals.clear()
    },
  }
}
