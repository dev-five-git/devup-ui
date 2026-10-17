import type { SourceType } from '@devup-ui/plugin-utils'

import type { ExtractResponse } from './coordinator-engine'
import type { ExtractRequest } from './coordinator-http'
import type { CoordinatorIdentity } from './coordinator-port'
import type { MdxInputFingerprint } from './mdx-source-freshness'
import type { CoordinatorInput } from './state'
import type { DevupWasm } from './wasm'

export interface CssQuery {
  fileNum?: number
  importMainCss: boolean
  wait: boolean
}

export interface CssResponse {
  css: string
  /** Which completeness guarantee the body carries. */
  policy: 'dev-current' | 'production-complete'
}

/** An app's coordinator without HTTP: what the endpoint calls. */
export interface Core {
  close(): void
  extract(request: ExtractRequest): Promise<ExtractResponse>
  css(query: CssQuery): Promise<CssResponse>
  /** Resume from the checkpoint and commit the first state. */
  startup(): Promise<void>
  /** Reconcile with disk outside any request (a watcher fired). */
  reconcile(changedPaths?: readonly string[]): Promise<void>
  watchInputs?(): readonly string[]
  onWatchInputs?(listener: (inputs: readonly string[]) => void): () => void
  /** Resolves once every accepted state is on disk. */
  flush(): Promise<void>
}

export interface PrewarmedOutput {
  code: string
  cssFile?: string
  map?: string
  source: string
  readonly sourceType?: SourceType
  updatedBaseStyle: boolean
  /** Files the extraction read through the module resolver */
  dependencies?: string[]
  readonly resolutionInputs?: readonly MdxInputFingerprint[]
}

export interface PreparedSourceEvidence {
  readonly compilerFingerprint: string
  readonly fileFingerprints: Readonly<Record<string, string>>
  readonly contextFingerprints: Readonly<Record<string, string>>
  readonly missingDependencies: readonly string[]
  readonly map?: string
}

export interface PreparedSource {
  /** Exact pre-Devup compiled JS/JSX under its real source identity. */
  readonly input: CoordinatorInput
  readonly evidence: PreparedSourceEvidence
}

export interface PreparedSourceGeneration {
  readonly sources: readonly PreparedSource[]
  readonly ordinaryInputs?: readonly CoordinatorInput[]
  readonly plan?: GenerationPlan
  readonly watchInputs?: readonly string[]
  readonly resolutionInputs?: readonly MdxInputFingerprint[]
  /** Captures this generation's configuration AND prepared-source resolver. */
  readonly configureWasm: (wasm: DevupWasm) => void
}

export interface GenerationPlan {
  readonly canonicalMap: Readonly<Record<string, string>>
  readonly expectedBaseFiles: readonly string[]
}

export interface ReplayExtractionReport {
  readonly filename: string
  readonly dependencies: readonly string[]
  readonly resolutionInputs?: readonly MdxInputFingerprint[]
}

export interface ReplayPreparation {
  readonly generation: PreparedSourceGeneration
  readonly signal: AbortSignal
  readonly changedPaths?: readonly string[]
  readonly observeExtraction?: (
    inputs: readonly CoordinatorInput[],
    configureWasm: PreparedSourceGeneration['configureWasm'],
  ) => readonly ReplayExtractionReport[]
}

export interface PreparedSources {
  readonly validateForCssFinalization?: (
    generation: PreparedSourceGeneration,
  ) => void
  readonly initial: {
    /** Ordinary inputs; compiled inputs belong to generation.sources. */
    readonly ordinaryInputs: readonly CoordinatorInput[]
    /** options.wasm already holds this complete generation, including empty. */
    readonly generation: PreparedSourceGeneration
    /** Includes allocator restoration and fresh prewarm changes. */
    readonly revision: number
  }
  /**
   * Runs inside the mutation queue before every dev CSS/extract/watch replay.
   * Return the supplied generation only after proving compiler/dependency
   * freshness; otherwise return a complete replacement, never stale bytes
   * stamped after compilation. Failures must reject and block the operation.
   * The provider owns compilation, dependency evidence and bounded cancellation.
   */
  readonly prepareReplay: (
    request: ReplayPreparation,
  ) => Promise<PreparedSourceGeneration>
}

export interface CoordinatorOptions {
  /** The engine serving requests until a rebuild swaps in a fresh one. */
  wasm: DevupWasm
  package: string
  cssDir: string
  singleCss: boolean
  /** @deprecated Persistence is the single `stateFile`; ignored. */
  sheetFile?: string
  /** @deprecated Persistence is the single `stateFile`; ignored. */
  classMapFile?: string
  /** @deprecated Persistence is the single `stateFile`; ignored. */
  fileMapFile?: string
  importAliases: Record<string, string | null>
  /** Where the endpoint is published; also the identity of this coordinator. */
  coordinatorPortFile: string
  /**
   * Canonical (single-importer collapse) map: cwd-relative POSIX source path ->
   * its canonical bucket path (or the `@global` sentinel). Used to wait for ALL
   * members of a shared CSS bucket before serving it. Empty when collapse is
   * disabled.
   */
  canonicalMap: Record<string, string>
  /**
   * Route-reachable source graph closure (cwd-relative POSIX): the files the
   * production base stylesheet needs before it can be served.
   */
  expectedBaseFiles?: string[]
  /**
   * Files a production prewarm extracted successfully. Providing it, even as
   * `[]`, declares the planned stylesheet complete once those files are in; it
   * is what makes a production CSS request servable without any loader POST.
   */
  prewarmedFiles?: string[]
  /**
   * Production outputs extracted before Turbopack starts. A loader that
   * sends byte-identical source gets the output back without a second
   * extraction; the engine already holds its styles.
   */
  prewarmedOutputs?: Map<string, PrewarmedOutput>
  /** Generate transform source maps. Defaults to true. */
  sourceMap?: boolean
  /**
   * Bound (ms) on how long a production `/css` waits for the planned files.
   * Past it the request fails naming what is missing. Defaults to 60000.
   */
  maxWaitMs?: number
  /** Directory relative paths resolve against. Defaults to `process.cwd()`. */
  projectRoot?: string
  /**
   * Ownership of this coordinator. When given, every request must carry the
   * matching `x-devup-project` / `x-devup-token` headers. Standalone callers
   * may omit it: a random identity is made and headers are optional.
   */
  identity?: CoordinatorIdentity
  /** Development mode: watch sources, reconcile deletions and config. */
  watch?: boolean
  /** The one file the engine state and input manifest are committed to. */
  stateFile?: string
  /** A checkpoint written for a different key is ignored (cold start). */
  optionsKey?: string
  /** Rewritten with a new revision after every CSS change; loaders watch it. */
  revisionFile?: string
  /** The `devup.json` whose theme (and `extends` chain) is applied. */
  devupFile?: string
  /** Directories whose changes can mean a source was deleted or renamed. */
  sourceRoots?: string[]
  /** Applied to every engine this coordinator builds (prefix, hoisting...). */
  configureWasm?: (wasm: DevupWasm) => void
  /** Makes a fresh engine for a rebuild. Defaults to `createWasm`. */
  createEngine?: () => DevupWasm
  /** Bound on cached extraction outputs. Defaults to 4096. */
  cacheMaxEntries?: number
  /** Complete fresh startup plus the compiler-owned development refresh seam. */
  readonly preparedSources?: PreparedSources
}

/** Ownership transport whose complete Core options will arrive asynchronously. */
export interface DeferredCoordinatorOptions {
  readonly projectRoot: string
  readonly identity: CoordinatorIdentity
  readonly coordinatorPortFile: string
  readonly prepare: (signal: AbortSignal) => Promise<CoordinatorOptions>
  /** Defaults to 50000ms, reserving the normal 60000ms CSS completion budget. */
  readonly maxPrepareMs?: number
}

export type CoordinatorStartOptions =
  CoordinatorOptions | DeferredCoordinatorOptions

export interface CoordinatorHandle {
  /** Resolves once the endpoint is published (or the coordinator was closed first). */
  readonly ready: Promise<void>
  /** Complete preparation and Core startup; optional for existing handle mocks. */
  readonly prepared?: Promise<void>
  /** Release this handle; the last one stops listening. Synchronous, no flushing. */
  close(): void
  /** Stop admitting requests, wait for accepted work and writes, then close. */
  drain(): Promise<void>
}

export interface PreparedCoordinatorHandle extends CoordinatorHandle {
  readonly prepared: Promise<void>
}
