export interface ResettableEngine {
  resetBuildState(): void
}

declare global {
  var __devupBuildOwnership: { running: number; generation: number } | undefined
}

const ownership = (globalThis.__devupBuildOwnership ??= {
  running: 0,
  generation: 0,
})

export function getBuildOwnership(): Readonly<typeof ownership> {
  return { ...ownership }
}

/**
 * Start a build in this process. The engine is shared by every plugin instance
 * in the process, so what an earlier build left in it (prefix, atom hoisting,
 * routes, buckets, names, numbers, styles) would otherwise leak into this one:
 * it is reset unless another build is still running, whose state it would
 * wipe. Returns the function that ends the build.
 */
export function beginBuild(engine: Partial<ResettableEngine>): () => void {
  if (ownership.running === 0) engine.resetBuildState?.()
  ownership.running += 1
  ownership.generation += 1
  let ended = false
  return () => {
    if (ended) return
    ended = true
    ownership.running -= 1
    ownership.generation += 1
  }
}
