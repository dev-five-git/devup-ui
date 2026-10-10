/** @internal An adapter to the existing synchronous engine snapshot API. */
export interface GenerationEngine<State> {
  reset(): void
  restore(state: State): void
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
 * @internal One integration-owned immutable handoff, never an engine lease.
 * Construction/configuration does not count as a running compiler. Only actual
 * run/watch participants count; an unused configuration cannot pin the owner.
 */
export class BuildGeneration<State> {
  private state: State | undefined
  private participants: number
  private finishing: boolean
  private closed: boolean

  constructor() {
    this.participants = 0
    this.finishing = false
    this.closed = false
  }

  get disposed(): boolean {
    return this.closed
  }

  run<T>(engine: GenerationEngine<State>, action: () => T): T {
    if (this.closed) throw new ClosedBuildGenerationError()
    engine.reset()
    if (this.state !== undefined) engine.restore(this.state)
    try {
      return action()
    } finally {
      if (!this.closed) this.state = engine.capture()
    }
  }

  acquire(complete: boolean): () => void {
    if (this.closed) throw new ClosedBuildGenerationError()
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
    this.closed = true
    this.state = undefined
  }
}
