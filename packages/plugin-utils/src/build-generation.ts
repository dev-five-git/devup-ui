import { resetOwnedBuildState } from './build-session'

export interface GenerationSite {
  readonly file: string
  readonly line: number
  readonly column: number
  readonly operation: string
}

export interface GenerationRun<T> {
  readonly site: GenerationSite
  readonly configure: () => void
  readonly action: () => T
}

/** @internal The binding owns identity and every state transition. */
export interface GenerationHandle {
  readonly disposed: boolean
}

export interface GenerationOwnership<State> {
  create(capture: () => State): GenerationHandle
  run<T>(handle: GenerationHandle, request: GenerationRun<T>): T
  dispose(handle: GenerationHandle): void
  clear(handle: GenerationHandle): void
}

/** @internal A required public ownership port, never a snapshot fallback. */
export interface GenerationEngine<State> {
  readonly ownership: GenerationOwnership<State>
  capture(): State
}

/** @internal Raised only for an operation after its integration closed. */
export class ClosedBuildGenerationError extends Error {
  constructor() {
    super('[devup-ui] build generation has already closed')
    this.name = 'ClosedBuildGenerationError'
  }
}

/**
 * @internal One integration's opaque Rust-owned generation, never an engine lease.
 * Construction/configuration does not count as a running compiler. Only actual
 * run/watch participants count; an unused configuration cannot pin the owner.
 */
export class BuildGeneration<State> {
  private binding:
    | {
        readonly port: GenerationOwnership<State>
        readonly handle: GenerationHandle
      }
    | undefined
  private participants: number
  private finishing: boolean
  private closed: boolean

  constructor() {
    this.participants = 0
    this.finishing = false
    this.closed = false
  }

  get disposed(): boolean {
    return this.closed || (this.binding?.handle.disposed ?? false)
  }

  run<T>(engine: GenerationEngine<State>, request: GenerationRun<T>): T {
    if (this.binding === undefined) {
      const handle = engine.ownership.create(() => engine.capture())
      this.binding = { port: engine.ownership, handle }
      if (this.closed) engine.ownership.dispose(handle)
    }
    return engine.ownership.run(this.binding.handle, request)
  }

  acquire(complete: boolean): () => void {
    if (this.disposed) throw new ClosedBuildGenerationError()
    this.participants += 1
    let released = false
    return () => {
      if (released) return
      released = true
      this.participants -= 1
      this.finishing ||= complete
      if (this.finishing && this.participants === 0) this.dispose()
    }
  }

  dispose(): void {
    if (this.closed) return
    this.closed = true
    if (this.binding !== undefined)
      this.binding.port.dispose(this.binding.handle)
    this.clear()
  }

  clear(): void {
    const binding = this.binding
    if (binding !== undefined)
      resetOwnedBuildState({
        resetBuildState: () => binding.port.clear(binding.handle),
      })
  }
}
