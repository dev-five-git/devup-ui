import type { DevupCompiler } from './compiler-types'

/** One app's engine session on a shared compiler. */
export interface CompilerSession {
  /** The engine session every call through `compiler` selects first. */
  readonly id: number
  /**
   * The shared compiler, scoped to this session: each function selects the
   * session, then runs. JavaScript is single-threaded and the calls are
   * synchronous, so no other session's call can run between the two.
   */
  readonly compiler: DevupCompiler
  /** Drop the session and its engine state; later calls through `compiler` throw. */
  close(): void
}

/**
 * Open an engine session of its own on `compiler`, so two apps in one process
 * never read or change each other's numbering, sheet, theme or settings.
 */
export function openCompilerSession(compiler: DevupCompiler): CompilerSession {
  const id = compiler.createSession()
  const scoped = new Map<PropertyKey, unknown>()
  return {
    id,
    compiler: new Proxy(compiler, {
      get(target, key) {
        const value: unknown = Reflect.get(target, key)
        if (typeof value !== 'function') return value
        let wrapper = scoped.get(key)
        if (!wrapper) {
          // Read the function at call time, so a later replacement applies
          wrapper = function (this: unknown, ...args: unknown[]) {
            target.selectSession(id)
            return Reflect.apply(Reflect.get(target, key), this, args)
          }
          scoped.set(key, wrapper)
        }
        return wrapper
      },
    }),
    close() {
      compiler.dropSession(id)
    },
  }
}
