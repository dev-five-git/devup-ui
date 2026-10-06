import type {
  ModuleAliases,
  PreparedSource,
  StaticImportGraph,
} from '@devup-ui/plugin-utils'

import type { PreparedSourceGeneration } from './coordinator-options'
import type { MdxPipeline } from './mdx-pipeline'
import type { PreparedMdx } from './mdx-prepare-result'
import type { MdxPreparationContext } from './mdx-prewarm-boundary'
import type { MdxInputFingerprint } from './mdx-source-freshness'
import type { SourcePlan } from './plan'
import type { AppContext } from './session'
import type { CoordinatorInput } from './state'
import type { DevupWasm, ModuleResolverSettings } from './wasm'

export type MdxPipelineSelection = {
  readonly pipeline: MdxPipeline
  readonly context: MdxPreparationContext
  /** Original pre-execution resolved module/version/hash/options facts. */
  readonly identity: unknown
}
export type MdxNativeExpectation = {
  readonly filename: string
  readonly reason: string
  readonly proof: object
}
export type MdxOrdinaryEligibility =
  | { readonly kind: 'disk-first' }
  | {
      readonly kind: 'native-required'
      readonly expectation: MdxNativeExpectation
    }

export type MdxSourcePlan = Omit<
  SourcePlan,
  'graph' | 'seedFiles' | 'expectedBaseFiles' | 'fileRoutes' | 'canonicalMap'
> & {
  readonly graph: Omit<
    StaticImportGraph,
    | 'files'
    | 'fileSet'
    | 'staticImports'
    | 'staticImporters'
    | 'dynamicImports'
    | 'dynamicTargets'
    | 'externalImports'
  > & {
    readonly files: readonly string[]
    readonly fileSet: ReadonlySet<string>
    readonly staticImports: ReadonlyMap<string, ReadonlySet<string>>
    readonly staticImporters: ReadonlyMap<string, ReadonlySet<string>>
    readonly dynamicImports: ReadonlyMap<string, ReadonlySet<string>>
    readonly dynamicTargets: ReadonlySet<string>
    readonly externalImports: ReadonlyMap<string, ReadonlySet<string>>
  }
  readonly seedFiles: readonly string[]
  readonly expectedBaseFiles: readonly string[]
  readonly fileRoutes: Readonly<Record<string, readonly number[]>>
  readonly canonicalMap: Readonly<Record<string, string>>
}

export type MdxExtractionReport = {
  readonly filename: string
  readonly dependencies: readonly string[]
}
export type MdxExtractionView = {
  readonly plan: MdxSourcePlan
  readonly resolver: ModuleResolverSettings
  readonly inputs: readonly CoordinatorInput[]
}
export type MdxBuildBinding = {
  readonly effectiveAppContext: AppContext
  readonly extensions: readonly string[]
  readonly aliases: ModuleAliases
  readonly conditions: readonly string[]
  readonly configFile: string
  readonly selectPipeline: (
    filename: string,
    signal: AbortSignal,
  ) => Promise<MdxPipelineSelection | undefined>
  readonly ordinaryEligibility: (filename: string) => MdxOrdinaryEligibility
  readonly configureWasm: (wasm: DevupWasm, view: MdxExtractionView) => void
  readonly extractDependencies: (
    view: MdxExtractionView,
    signal: AbortSignal,
  ) => Promise<readonly MdxExtractionReport[]>
}

export type MdxPreparationRun = {
  readonly changedPaths?: readonly string[]
  readonly extractDependencies?: MdxBuildBinding['extractDependencies']
}

export type MdxCompiledRecord = {
  readonly prepared: PreparedMdx
  readonly inputs: readonly MdxInputFingerprint[]
}
export type MdxSourceGeneration = PreparedSourceGeneration &
  MdxExtractionView & {
    readonly key: object
    readonly compiled: Readonly<Record<string, MdxCompiledRecord>>
    readonly ordinaryInputs: readonly CoordinatorInput[]
    readonly pendingOrdinary: readonly MdxNativeExpectation[]
    readonly cacheReader: (filename: string) => PreparedSource
    readonly watchInputs: readonly string[]
    readonly extractionDependencies: readonly MdxExtractionReport[]
  }
