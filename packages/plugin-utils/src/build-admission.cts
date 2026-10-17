/** @internal Metadata supplied by the integration at its actual boundary. */
export interface BuildIntegration {
  readonly integration: string
  readonly root: string
}

export interface ResettableEngine {
  resetBuildState(): void
}

/** @internal Mixed opaque legacy state cannot be replayed by an owner. */
export class MixedBuildIntegrationError extends Error {
  constructor(
    readonly requested: BuildIntegration,
    readonly active: BuildIntegration,
  ) {
    super(
      `[devup-ui] ${requested.integration} at ${requested.root}: cannot activate while ${active.integration} at ${active.root} is using the shared build engine; run them in separate processes`,
    )
    this.name = 'MixedBuildIntegrationError'
  }
}

// Only admission metadata lives here, never engine state or generation owners.
const legacy = new Set<{ readonly context: BuildIntegration }>()
const operations: BuildIntegration[] = []

/** Start a legacy interval; overlapping legacy intervals retain their ABI. */
export function beginBuild(
  engine: Partial<ResettableEngine>,
  context: BuildIntegration = {
    integration: 'legacy beginBuild',
    root: process.cwd(),
  },
): () => void {
  const active = operations.at(-1)
  if (active) throw new MixedBuildIntegrationError(context, active)
  if (legacy.size === 0) engine.resetBuildState?.()
  const interval = { context }
  legacy.add(interval)
  return () => {
    legacy.delete(interval)
  }
}

/** @internal Admit before any owner reset, restore, configuration or action. */
export function runBuildOperation<T>(
  context: BuildIntegration,
  action: () => T,
): T {
  const active = legacy.values().next().value
  if (active) throw new MixedBuildIntegrationError(context, active.context)
  operations.push(context)
  try {
    return action()
  } finally {
    operations.pop()
  }
}

/** @internal Cleanup must not reset an opaque live legacy configuration. */
export function resetOwnedBuildState(engine: ResettableEngine): void {
  if (legacy.size === 0) engine.resetBuildState()
}

export default {
  beginBuild,
  MixedBuildIntegrationError,
  resetOwnedBuildState,
  runBuildOperation,
}
