export { BuildGeneration, type GenerationEngine } from './build-generation'
export {
  beginBuild,
  type BuildIntegration,
  getBuildOwnership,
  MixedBuildIntegrationError,
  resetOwnedBuildState,
  type ResettableEngine,
  runBuildOperation,
} from './build-session'
export {
  type CompiledReference,
  createCompileTimeClassifier,
  isCompileTimeAlias,
  UntransformedSourceError,
} from './compiled-guard'
export {
  createDependencyGuard,
  type DependencyGuardOptions,
  type GuardModule,
} from './dependency-guard'
export {
  compiledFacts,
  type GuardFacts,
  type GuardObservation,
} from './guard-facts'
export {
  type AtomHoistPlan,
  buildCanonicalMap,
  type BuildCanonicalMapOptions,
  buildPreparationImportGraph,
  buildStaticImportGraph,
  computeCompiledFiles,
  type ComputeCompiledFilesOptions,
  computeFileReach,
  type ComputeFileReachOptions,
  computeFileRoutes,
  type ComputeFileRoutesOptions,
  computeReachableFiles,
  type ComputeReachableFilesOptions,
  createModulePathResolver,
  createModuleResolver,
  type CreateModuleResolverOptions,
  type IgnoredModule,
  listSourceFiles,
  type ModulePathResolution,
  type ModulePathResolver,
  type ModulePathResolverOptions,
  type ModuleResolution,
  parseImports,
  planAtomHoist,
  type PreparedGraphOptions,
  type PreparedSource,
  type PrepareSource,
  type ResolvedModule,
  type ScannedStaticImportGraph,
  type StaticImportGraph,
  type StaticImportGraphOptions,
  type SyncGraphOptions,
} from './import-graph'
export {
  importGraphFailureOf,
  type ImportGraphRequest,
  type ImportRequestOutcome,
} from './import-requests'
export { type ImportReference, scanImports } from './import-scanner'
export { readJsxImportSource } from './jsx-import-source'
export {
  ConfigLoadError,
  deepMerge,
  loadDevupConfig,
  loadDevupConfigSync,
} from './load-config'
export { remapMdxError } from './mdx-errors'
export {
  isMdxSource,
  mdxSourceFilter,
  normalizeMdxExtensions,
  selectedSourceFilter,
} from './mdx-selection'
export {
  nextNonphysicalIdList,
  type NonphysicalIdListOutcome,
} from './nonphysical-id-list'
export {
  createNonphysicalIdStore,
  type NonphysicalIdStore,
  type NonphysicalIdStoreScope,
} from './nonphysical-id-store'
export {
  type NonphysicalModuleId,
  type NonphysicalRequest,
  normalizeNonphysicalModuleId,
  type PhysicalImporter,
} from './nonphysical-module-id'
export {
  collectNumberedFiles,
  type CollectNumberedFilesOptions,
  extractedNeedles,
  type FileNumbering,
  isProjectJsx,
  seedFileNumbers,
} from './numbering'
export { PreparationInputs } from './preparation-inputs'
export type { ModuleResolver } from './prepared-resolver'
export type { SourceType } from './prepared-source'
export {
  collectProductionFileManifest,
  type ProductionFileManifestOptions,
  type ProductionManifestContext,
  type ProductionManifestFile,
} from './production-file-manifest'
export {
  type ProductionExtractionInput,
  ProductionNumbering,
  type ProductionNumberingEngine,
  ProductionNumberingError,
  type ProductionNumberingPlan,
} from './production-numbering'
export {
  enumerateProductionSourceFiles,
  type ProductionSourceFile,
  type ProductionSourceFilesOptions,
} from './production-source-files'
export {
  collectReservationInputs,
  PreparedInputConflictError,
  type PreparedReservationInput,
  type ReservationDiscoveryOptions,
  type ReservationInput,
  type ReservationInventory,
} from './reservation-discovery'
export { reservationGraph } from './reservation-graph'
export {
  type ResolutionInputObserver,
  type ResolutionInputs,
} from './resolution-inputs'
export { resolutionWatchPath } from './resolution-watch-path'
export {
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  DEFAULT_THEME_INTERFACE_NAMES,
  type DevupThemeInterfaceNames,
  type DevupUIBasePluginOptions,
  getFileNumByFilename,
  GRAPH_SOURCE_FILE_RE,
  MDX_FILE_RE,
  POST_COMPILED_MDX_RE,
  resolveProjectPaths,
  resolveSourceDirs,
  SOURCE_EXTENSIONS,
  SOURCE_FILE_RE,
} from './shared'
export {
  createSharedSlotReservation,
  type ReservationBatchOptions,
  type ReservationEngine,
  type ReservationRow,
  type SharedSlotReservation,
} from './shared-slots'
export {
  isSelectedSource,
  type MdxSelection,
  type SourceSelectionOptions,
} from './source-selection'
export {
  createStateWriter,
  type StateWriter,
  writeFileAtomically,
} from './state-writer'
export {
  findTailwindCss,
  TAILWIND_CSS_CANDIDATES,
  tailwindCssFiles,
  type TailwindCssSource,
  withTailwindCss,
} from './tailwind-css'
export type {
  CustomShorthands,
  DevupConfig,
  DevupTheme,
  ImportAliases,
  ModuleAliasDescriptor,
  ModuleAliases,
  ModuleAliasOptions,
  ThemeColors,
  ThemeTypography,
  Typography,
  WasmImportAliases,
} from './types'
export {
  createCompatTypes,
  DEFAULT_IMPORT_ALIASES,
  mergeImportAliases,
} from './types'
