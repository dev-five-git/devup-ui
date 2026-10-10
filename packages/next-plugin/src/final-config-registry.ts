import type { NextConfig } from 'next'

export interface FinalConfigContext {
  readonly phase: string
  readonly nextVersion: string
  readonly projectDir: string
}

/** Turbo awaits preparation here; webpack only registers its beforeCompile bridge. */
export type FinalConfigFinalizer = (
  config: NextConfig,
  context: FinalConfigContext,
  signal: AbortSignal,
) => NextConfig | Promise<NextConfig>

export interface FinalConfigRegistration {
  readonly token: string
  readonly projectDir: string
  readonly configFile: string
  readonly adapterPath: string
  readonly callerPath: string | undefined
  readonly timeoutMs: number
  readonly finalize: FinalConfigFinalizer
}

export interface FinalConfigRecord extends FinalConfigRegistration {
  readonly controllers: Map<AbortController, FinalConfigContext>
}

const FINALIZERS = Symbol.for('@devup-ui/next-plugin/final-config-finalizers')

/** Bundled config reloads and the standalone CJS entry share live callbacks here. */
export function finalConfigRecords(): Map<string, FinalConfigRecord> {
  const holder: typeof globalThis & {
    [FINALIZERS]?: Map<string, FinalConfigRecord>
  } = globalThis
  return (holder[FINALIZERS] ??= new Map())
}

export class FinalConfigAdapterError extends Error {
  readonly name = 'FinalConfigAdapterError'
  readonly stage: string

  constructor(
    readonly context: FinalConfigContext,
    readonly location: string,
    options: ErrorOptions & { readonly stage: string },
  ) {
    const detail =
      options.cause instanceof Error
        ? options.cause.message
        : String(options.cause)
    super(
      `${location}:1:1: devup-ui cannot use \`Next final config\` at build time: ${options.stage} (${context.phase}, project ${context.projectDir}): ${detail}; needs a live session and a successful bounded caller adapter/finalizer.`,
      options,
    )
    this.stage = options.stage
  }
}
