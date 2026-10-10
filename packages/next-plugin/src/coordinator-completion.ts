export class IncompleteCssError extends Error {
  constructor(
    readonly label: string,
    readonly missing: readonly string[],
    readonly failed: ReadonlyMap<string, string>,
  ) {
    const first = failed.keys().next().value ?? missing[0]
    const parts = [
      ...[...failed].map(
        ([file, reason]) => `extraction failed for ${file}: ${reason}`,
      ),
      ...(missing.length > 0 ? [`never extracted: ${missing.join(', ')}`] : []),
    ]
    super(
      `${first}:1:1: devup-ui coordinator cannot use \`${label}\` at build time: ${parts.join('; ')}. Fix: make every listed file transform through the devup-ui loader (or drop it from the planned set) and rerun; partial CSS is never served.`,
    )
  }
}

export class CoordinatorShutdownError extends Error {
  constructor(readonly file: string) {
    super(
      `${file}:1:1: devup-ui coordinator cannot use \`request\` at build time: it is shutting down; restart the dev server or build.`,
    )
  }
}

export interface CompletionTracker {
  close(): void
  succeed(file: string): void
  fail(file: string, message: string): void
  forget(file: string): void
  /**
   * Resolves once every file succeeded. Rejects as soon as one has failed, or
   * with the files still missing after `maxWaitMs`.
   */
  wait(label: string, files: readonly string[]): Promise<void>
}

export function createCompletionTracker(maxWaitMs: number): CompletionTracker {
  const done = new Set<string>()
  const failures = new Map<string, string>()
  const waiters = new Map<() => void, () => void>()
  let closed = false
  const settle = () => {
    for (const waiter of waiters.keys()) waiter()
  }
  return {
    close() {
      closed = true
      for (const cancel of waiters.values()) cancel()
    },
    succeed(file) {
      done.add(file)
      failures.delete(file)
      settle()
    },
    fail(file, message) {
      done.delete(file)
      failures.set(file, message)
      settle()
    },
    forget(file) {
      done.delete(file)
      failures.delete(file)
    },
    wait(label, files) {
      if (closed)
        return Promise.reject(new CoordinatorShutdownError('devup-ui.css'))
      return new Promise((resolve, reject) => {
        const check = (expired: boolean) => {
          const failed = new Map(
            files
              .filter((file) => failures.has(file))
              .map((file) => [file, failures.get(file) ?? ''] as const),
          )
          const missing = files.filter(
            (file) => !done.has(file) && !failures.has(file),
          )
          if (failed.size === 0 && missing.length === 0) {
            finish()
            resolve()
          } else if (failed.size > 0 || expired) {
            finish()
            reject(new IncompleteCssError(label, missing, failed))
          }
        }
        const waiter = () => check(false)
        const timer = setTimeout(() => check(true), maxWaitMs)
        const finish = () => {
          clearTimeout(timer)
          waiters.delete(waiter)
        }
        waiters.set(waiter, () => {
          finish()
          reject(new CoordinatorShutdownError('devup-ui.css'))
        })
        check(false)
      })
    },
  }
}
