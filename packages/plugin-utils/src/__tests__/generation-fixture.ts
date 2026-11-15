import { fileURLToPath } from 'node:url'

import {
  ClosedBuildGenerationError,
  type GenerationHandle,
  type GenerationOwnership,
  type GenerationRun,
} from '../build-generation'

export function operation<T>(action: () => T): GenerationRun<T> {
  return {
    site: {
      file: fileURLToPath(import.meta.url),
      line: 1,
      column: 1,
      operation: 'test snapshot transport',
    },
    configure: () => undefined,
    action,
  }
}

interface SnapshotEngine<State> {
  reset(): void
  restore(state: State): void
  capture(): State
}

export function snapshotEngine<State, Engine extends SnapshotEngine<State>>(
  engine: Engine & SnapshotEngine<State>,
): Engine & { readonly ownership: GenerationOwnership<State> } {
  const entries = new Map<
    GenerationHandle,
    { readonly capture: () => State; state: State | undefined }
  >()
  const disposed = new WeakSet<GenerationHandle>()
  const entry = (handle: GenerationHandle) => {
    const selected = entries.get(handle)
    if (selected === undefined) throw new TypeError('foreign test handle')
    return selected
  }
  return {
    ...engine,
    ownership: {
      create(capture) {
        const handle: GenerationHandle = {
          get disposed() {
            return disposed.has(handle)
          },
        }
        entries.set(handle, { capture, state: undefined })
        return handle
      },
      run(handle, request) {
        if (disposed.has(handle)) throw new ClosedBuildGenerationError()
        const selected = entry(handle)
        engine.reset()
        if (selected.state !== undefined) engine.restore(selected.state)
        try {
          request.configure()
          return request.action()
        } finally {
          if (!disposed.has(handle)) selected.state = selected.capture()
        }
      },
      dispose(handle) {
        entry(handle)
        disposed.add(handle)
      },
      clear(handle) {
        entry(handle).state = undefined
      },
    },
  }
}
