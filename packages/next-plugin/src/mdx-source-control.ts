import { createMdxDeadline, type MdxDeadline } from './mdx-prepare'
import { MdxFreshnessError } from './mdx-source-freshness'

export async function withMdxSourceControl<T>(
  request: {
    readonly filename: string
    readonly signal: AbortSignal
    readonly deadline?: MdxDeadline
  },
  work: (signal: AbortSignal, deadline: MdxDeadline) => Promise<T>,
): Promise<T> {
  request.signal.throwIfAborted()
  const deadline = request.deadline ?? createMdxDeadline()
  const controller = new AbortController()
  const propagate = () => controller.abort(request.signal.reason)
  request.signal.addEventListener('abort', propagate, { once: true })
  const timer = setTimeout(
    () =>
      controller.abort(
        new MdxFreshnessError(
          request.filename,
          { kind: 'file', path: request.filename, loader: 'generation' },
          'preparation deadline exceeded',
        ),
      ),
    Math.max(0, deadline.expiresAt - Date.now()),
  )
  const aborted = Promise.withResolvers<never>()
  const reject = () => aborted.reject(controller.signal.reason)
  controller.signal.addEventListener('abort', reject, { once: true })
  try {
    return await Promise.race([
      work(controller.signal, deadline),
      aborted.promise,
    ])
  } finally {
    clearTimeout(timer)
    request.signal.removeEventListener('abort', propagate)
    controller.signal.removeEventListener('abort', reject)
  }
}
