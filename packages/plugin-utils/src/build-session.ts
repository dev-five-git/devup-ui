export interface ResettableEngine {
  resetBuildState(): void
}

let running = 0

/**
 * Start a build in this process. The engine is shared by every plugin instance
 * in the process, so what an earlier build left in it (prefix, atom hoisting,
 * routes, buckets, names, numbers, styles) would otherwise leak into this one:
 * it is reset unless another build is still running, whose state it would
 * wipe. Returns the function that ends the build.
 */
export function beginBuild(engine: Partial<ResettableEngine>): () => void {
  if (running === 0) engine.resetBuildState?.()
  running += 1
  let ended = false
  return () => {
    if (ended) return
    ended = true
    running -= 1
  }
}
