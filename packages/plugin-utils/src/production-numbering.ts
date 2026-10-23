import {
  type BuildGeneration,
  ClosedBuildGenerationError,
} from './build-generation'
import type { FileNumbering } from './numbering'
import type { ProductionManifestFile } from './production-file-manifest'

export interface ProductionNumberingPlan {
  readonly context: string
  readonly files: readonly ProductionManifestFile[]
  /** Already normalized by the caller; reservation adds no discovery or append policy. */
  readonly nonphysical: readonly string[]
  readonly canonical: Readonly<Record<string, string>>
}

export interface ProductionNumberingEngine extends FileNumbering {
  importCanonicalMap(map: Readonly<Record<string, string>>): void
}

export interface ProductionExtractionInput {
  readonly context: string
  readonly id: string
  readonly source: {
    readonly kind: 'physical'
    readonly path: string
    readonly realPath: string
  }
  readonly location: {
    readonly filename: string
    readonly importer?: string
    readonly line?: number
    readonly column?: number
  }
}

type Reason = 'physical-miss' | 'seed-failure' | 'invalid-plan' | 'unseeded'
interface ErrorDetails {
  readonly context?: string
  readonly id?: string
  readonly location?: ProductionExtractionInput['location']
  readonly cause?: unknown
  readonly restoreCause?: unknown
}
const messages = {
  'physical-miss':
    'physical source was not reserved; fix production discovery before extraction',
  'seed-failure': 'production reservation failed; abort the owning build',
  'invalid-plan':
    'production reservation plan has inconsistent context authority or provenance',
  unseeded: 'production reservation must finish before physical extraction',
} as const

export class ProductionNumberingError extends Error {
  readonly name = 'ProductionNumberingError'
  readonly context: string | undefined
  readonly id: string | undefined
  readonly location: ProductionExtractionInput['location'] | undefined
  readonly restoreCause?: unknown

  constructor(
    readonly reason: Reason,
    details: ErrorDetails = {},
  ) {
    const location =
      details.location === undefined
        ? undefined
        : Object.freeze({
            ...details.location,
            line: details.location.line ?? 1,
            column: details.location.column ?? 1,
          })
    const prefix =
      location === undefined
        ? '[devup-ui]'
        : `${location.filename}:${location.line}:${location.column}:`
    super(
      `${prefix} ${messages[reason]} (context=${details.context ?? 'unspecified'}, id=${details.id ?? 'unspecified'})`,
      'cause' in details ? { cause: details.cause } : undefined,
    )
    this.context = details.context
    this.id = details.id
    this.location = location
    if ('restoreCause' in details) this.restoreCause = details.restoreCause
  }
}

type Phase =
  | { readonly kind: 'unseeded' }
  | { readonly kind: 'seeded' }
  | { readonly kind: 'failed'; readonly error: ProductionNumberingError }

/**
 * Dormant reservations, constructed after all discovery/canonical plans close.
 * One primitive per owner is a caller precondition. Seed the direct engine inside
 * an admitted, restored, configured synchronous operation before ANY allocation.
 * Seed success must reach that same owner's successful capture; on capture failure
 * the caller aborts/disposes. No rollback or post-capture handshake is provided.
 * This reserves file identities, not differing-value class order within one bucket.
 */
export class ProductionNumbering {
  readonly files: readonly ProductionManifestFile[]
  readonly buckets: readonly string[]
  private readonly contexts = new Map<
    string,
    Readonly<Record<string, string>>
  >()
  private readonly memberships = new Map<string, ProductionManifestFile>()
  private phase: Phase = { kind: 'unseeded' }

  constructor(
    private readonly owner: Pick<BuildGeneration<unknown>, 'disposed'>,
    plans: readonly ProductionNumberingPlan[],
  ) {
    if (owner.disposed) throw new ClosedBuildGenerationError()
    const buckets = new Set<string>()
    for (const plan of plans) {
      const context = plan.context
      if (this.contexts.has(context))
        throw new ProductionNumberingError('invalid-plan', { context })
      const canonical = Object.freeze({ ...plan.canonical })
      this.contexts.set(context, canonical)
      for (const input of plan.files) {
        const file = Object.freeze({ ...input })
        const key = JSON.stringify([context, file.id])
        const previous = this.memberships.get(key)
        if (
          file.context !== context ||
          (previous !== undefined &&
            (previous.path !== file.path ||
              previous.realPath !== file.realPath))
        )
          throw new ProductionNumberingError('invalid-plan', {
            context,
            id: file.id,
          })
        this.memberships.set(key, file)
      }
      for (const id of [
        ...plan.files.map((file) => file.id),
        ...plan.nonphysical,
      ]) {
        buckets.add(id)
        buckets.add(Object.hasOwn(canonical, id) ? (canonical[id] ?? id) : id)
      }
    }
    this.files = Object.freeze([...this.memberships.values()])
    this.buckets = Object.freeze([...buckets])
  }

  private seeded(): boolean {
    if (this.owner.disposed) throw new ClosedBuildGenerationError()
    const phase = this.phase
    switch (phase.kind) {
      case 'unseeded':
        return false
      case 'seeded':
        return true
      case 'failed':
        throw phase.error
      default:
        return phase satisfies never
    }
  }

  seed(engine: ProductionNumberingEngine, activeContext: string): void {
    const seeded = this.seeded()
    const canonical = this.contexts.get(activeContext)
    if (canonical === undefined)
      throw new ProductionNumberingError('invalid-plan', {
        context: activeContext,
      })
    if (seeded) return
    let failure: ProductionNumberingError | undefined
    try {
      engine.importCanonicalMap({})
      engine.seedFileMap([...this.buckets])
    } catch (cause) {
      failure = new ProductionNumberingError('seed-failure', {
        context: activeContext,
        cause,
      })
    } finally {
      try {
        engine.importCanonicalMap(canonical)
      } catch (restoreCause) {
        failure = new ProductionNumberingError('seed-failure', {
          context: activeContext,
          cause: failure === undefined ? restoreCause : failure.cause,
          restoreCause,
        })
      }
    }
    if (failure !== undefined) {
      this.phase = { kind: 'failed', error: failure }
      throw failure
    }
    this.phase = { kind: 'seeded' }
  }

  /** Invoke before direct/prewarm extraction and before returning each physical child to WASM. */
  assertReserved(input: ProductionExtractionInput): void {
    if (!this.seeded()) throw new ProductionNumberingError('unseeded', input)
    const file = this.memberships.get(JSON.stringify([input.context, input.id]))
    if (
      file === undefined ||
      file.path !== input.source.path ||
      file.realPath !== input.source.realPath
    )
      throw new ProductionNumberingError('physical-miss', input)
  }
}
