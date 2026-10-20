import { once } from 'node:events'

import { processRuntime } from './process-inspection.mjs'
import { ProcessOwnership } from './process-ownership.mjs'

export class CommandExitError extends Error {
  constructor(result) {
    super(`${result.command} exited ${result.code ?? result.signal}`)
    this.result = result
  }
}

function bounded(operation, timeoutMs) {
  let timer
  return Promise.race([
    Promise.resolve().then(operation),
    new Promise((_, reject) => {
      timer = setTimeout(
        () => reject(new Error('Process inspection/cleanup timed out')),
        timeoutMs,
      )
    }),
  ]).finally(() => clearTimeout(timer))
}

export async function runGuarded(command, args, options = {}) {
  const io = options.runtime ?? processRuntime
  const operationTimeoutMs = options.drainTimeoutMs ?? 5000
  const runtime = Object.fromEntries(
    ['inspect', 'snapshot', 'privateBytes', 'kill'].map((key) => [
      key,
      (...values) => bounded(() => io[key](...values), operationTimeoutMs),
    ]),
  )
  const child = io.spawn(command, args, {
    cwd: options.cwd,
    env: { ...process.env, CARGO_BUILD_JOBS: '2', ...options.env },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  // Observe the error/close promise immediately, including spawn failures.
  const completion = once(child, 'close').then(
    ([code, signal]) => ({ code, signal }),
    (error) => ({ error }),
  )
  for (const stream of [child.stdout, child.stderr]) {
    stream.on('data', (data) => {
      ;(stream === child.stdout ? process.stdout : process.stderr).write(data)
      options.onOutput?.(data.toString())
    })
  }
  const owned = new ProcessOwnership(runtime)
  const started = performance.now()
  const errors = []
  let peakBytes = 0
  let stopReason
  let timer
  let pendingPoll = Promise.resolve()
  let outcome = {}
  let stopped = false
  let wake
  const failure = new Promise((resolve) => {
    wake = resolve
  })
  const poll = async () => {
    const rows = await runtime.snapshot()
    if (stopped) return
    await owned.discover(rows)
    const bytes = await owned.memory()
    peakBytes = Math.max(peakBytes, bytes)
    if (bytes > (options.memoryLimit ?? 6 * 1024 ** 3))
      stopReason = `private memory exceeded ${options.memoryLimit ?? 6 * 1024 ** 3} bytes`
    if (performance.now() - started > (options.timeoutMs ?? 600_000))
      stopReason = 'command timeout'
    if (options.signal?.aborted) stopReason = 'command cancelled'
    if (stopReason) wake()
  }
  const schedule = () => {
    // This handler is attached in the SAME turn, never after child completion.
    pendingPoll = poll().catch((error) => {
      errors.push(error)
      wake()
    })
    pendingPoll.then(() => {
      if (!stopped && !errors.length && !stopReason)
        timer = setTimeout(schedule, options.pollIntervalMs ?? 1000)
    })
  }
  try {
    // Establish exactly once, before the first fallible global snapshot.
    if (child.pid !== undefined) await owned.establish(child.pid)
    schedule()
    const completed = await Promise.race([completion, failure])
    if (completed) outcome = completed
    if (outcome.error) errors.push(outcome.error)
  } catch (error) {
    errors.push(error)
  } finally {
    stopped = true
    owned.closed = true
    clearTimeout(timer)
    try {
      await bounded(() => pendingPoll, operationTimeoutMs)
    } catch (error) {
      errors.push(error)
    } finally {
      // Known identities do not depend on another global snapshot succeeding.
      errors.push(...(await owned.stop(operationTimeoutMs)))
      try {
        outcome = await bounded(() => completion, operationTimeoutMs)
        if (outcome.error && !errors.includes(outcome.error))
          errors.push(outcome.error)
      } catch (error) {
        errors.push(error)
        child.stdout.destroy?.()
        child.stderr.destroy?.()
      }
    }
  }
  const result = {
    command,
    args,
    code: outcome.code,
    signal: outcome.signal,
    peakBytes,
    durationMs: performance.now() - started,
    stopReason,
    errors: errors.map((error) =>
      error instanceof Error ? error.message : String(error),
    ),
  }
  try {
    options.onResult?.(result)
  } catch (error) {
    errors.push(error)
  }
  if (errors.length)
    throw new AggregateError(errors, `Supervisor failed: ${command}`)
  if (stopReason)
    throw new Error(`FINDING: watchdog stopped ${command}: ${stopReason}`)
  if (result.code !== 0) throw new CommandExitError(result)
  return result
}
