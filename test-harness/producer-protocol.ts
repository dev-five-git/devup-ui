import { CoverageError } from './lcov'
import {
  array,
  integer,
  object,
  type ScriptCapture,
  text,
} from './producer-data'

const NativeWebSocket = globalThis.WebSocket

interface PendingRequest {
  readonly complete: (value: unknown) => void
  readonly reject: (error: Error) => void
  readonly timeout: ReturnType<typeof setTimeout>
}

export class ProducerProtocol {
  private sequence = 0
  private readonly pending = new Map<number, PendingRequest>()
  readonly events: Record<string, unknown>[] = []
  onEvent: ((event: Record<string, unknown>) => void) | undefined

  constructor(private readonly socket: WebSocket) {
    socket.addEventListener('message', (event) => {
      try {
        const message = object(JSON.parse(String(event.data)))
        if (message['id'] !== undefined) {
          const id = integer(message['id'])
          const request = this.pending.get(id)
          if (!request)
            throw new CoverageError('unexpected producer RPC response')
          this.pending.delete(id)
          clearTimeout(request.timeout)
          request.complete(message)
        } else {
          this.events.push(message)
          this.onEvent?.(message)
        }
      } catch (error) {
        if (!(error instanceof SyntaxError || error instanceof CoverageError))
          throw error
        this.fail(
          new CoverageError(`invalid producer RPC message: ${error.message}`),
        )
      }
    })
    socket.addEventListener(
      'error',
      this.fail.bind(this, new CoverageError('producer socket error')),
    )
    socket.addEventListener(
      'close',
      this.fail.bind(this, new CoverageError('producer socket closed')),
    )
  }

  private fail(error: CoverageError): void {
    for (const request of this.pending.values()) {
      clearTimeout(request.timeout)
      request.reject(error)
    }
    this.pending.clear()
    this.socket.close()
  }

  async request(
    method: string,
    params: Readonly<Record<string, unknown>> = {},
  ): Promise<unknown> {
    const id = ++this.sequence
    const response = object(
      await new Promise<unknown>((resolve, reject) => {
        const timeout = setTimeout(() => {
          this.pending.delete(id)
          reject(new CoverageError(`producer RPC timeout: ${method}`))
        }, 10000)
        this.pending.set(id, { complete: resolve, reject, timeout })
        this.socket.send(JSON.stringify({ id, method, params }))
      }),
    )
    if (response['error'] !== undefined)
      throw new CoverageError(
        `producer RPC rejected: ${method}: ${JSON.stringify(response['error'])}`,
      )
    return response['result']
  }

  close(): void {
    this.socket.close()
  }

  async armEndFence(
    captures: readonly ScriptCapture[],
    preload: string,
  ): Promise<void> {
    await this.request('Debugger.setBreakpointsActive', { active: true })
    const armed = new Set<string>()
    for (const capture of captures) {
      if (capture.url === preload) continue
      const codeLines = capture.code.split('\n')
      const locations = object(
        await this.request('Debugger.getBreakpointLocations', {
          start: { scriptId: capture.scriptId, lineNumber: 0, columnNumber: 0 },
          end: {
            scriptId: capture.scriptId,
            lineNumber: codeLines.length - 1,
            columnNumber: codeLines.at(-1)?.length ?? 0,
          },
        }),
      )
      for (const item of array(locations['locations'])) {
        const location = object(item)
        if (text(location['scriptId']) !== capture.scriptId)
          throw new CoverageError('contradictory producer fence identity')
        const point = {
          scriptId: capture.scriptId,
          lineNumber: integer(location['lineNumber']),
          columnNumber: integer(location['columnNumber']),
        }
        const key = JSON.stringify(point)
        if (armed.has(key)) continue
        armed.add(key)
        await this.request('Debugger.setBreakpoint', { location: point })
      }
    }
  }
}

export async function connectProducer(url: string): Promise<ProducerProtocol> {
  const socket = new NativeWebSocket(url)
  const protocol = new ProducerProtocol(socket)
  await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(
      reject.bind(undefined, new CoverageError('producer connection timeout')),
      10000,
    )
    socket.addEventListener(
      'open',
      () => {
        clearTimeout(timeout)
        resolve()
      },
      { once: true },
    )
    socket.addEventListener(
      'error',
      () => {
        clearTimeout(timeout)
        reject(new CoverageError('producer connection failed'))
      },
      { once: true },
    )
  })
  try {
    await protocol.request('Runtime.enable')
    await protocol.request('Debugger.enable')
    return protocol
  } catch (error) {
    protocol.close()
    throw error
  }
}
