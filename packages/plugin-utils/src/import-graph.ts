import {
  existsSync,
  readdirSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, isAbsolute, join, relative, resolve } from 'node:path'

import { createDirectoryExclusion } from './directory-exclusion'
import {
  freezeImportRequests,
  type ImportGraphRequest,
  type ImportRequestOutcome,
  retainImportGraphFailure,
} from './import-requests'
import {
  type ImportRequestReference,
  scanImportRequests,
} from './import-scanner'
import { remapMdxError } from './mdx-errors'
import { type AliasResolution, resolveModuleAlias } from './module-alias'
import { resolveFile } from './owned-file-resolution'
import { findPackage, resolvePackage } from './owned-module-resolution'
import { isResolutionFile as isFile } from './packaged-config-inputs'
import { preparedDiagnostics } from './prepared-diagnostics'
import {
  createPreparedResolver,
  type ModuleResolver,
} from './prepared-resolver'
import { PreparedSourceTypeError, readPreparedSource } from './prepared-source'
import {
  createResolutionInputs,
  type ResolutionInputObserver,
} from './resolution-inputs'
import { createNodeModulesExcludeRegex, SOURCE_FILE_RE } from './shared'
import {
  isSelectedSource,
  type MdxSelection,
  sourceExtensions,
  type SourceSelectionOptions,
} from './source-selection'
import { type PathAlias, readPathAliases } from './tsconfig'
import type { ModuleAliasOptions } from './types'

/**
 * How map keys (and bucket-root values) are stringified.
 * - `cwd-relative` (default): POSIX path relative to `cwd` — matches plugins
 *   that pass a cwd-relative filename to `codeExtract` (e.g. next-plugin).
 * - `absolute`: POSIX absolute path — matches plugins that pass the absolute
 *   module id to `codeExtract` (e.g. vite-plugin). Using the wrong mode makes
 *   the engine's bucket lookup miss, silently disabling collapse/hoisting.
 */
export type GraphKeyMode = 'cwd-relative' | 'absolute'

function makeToKey(cwd: string, keyBy: GraphKeyMode): (file: string) => string {
  return keyBy === 'absolute'
    ? (file: string) => file.replaceAll('\\', '/')
    : (file: string) => toPosixRelative(cwd, file)
}

export interface BuildCanonicalMapOptions {
  srcDir: string | string[]
  tsconfigPath?: string
  cwd: string
  hoistV?: number
  keyBy?: GraphKeyMode
  /** pre-built graph from `buildStaticImportGraph` to skip the file scan. */
  graph?: StaticImportGraph
}

interface OxcParser {
  parseSync: (
    filename: string,
    source: string,
    options?: Record<string, unknown>,
  ) => unknown
}

interface ResolveContext {
  aliases: PathAlias[]
  aliasBaseDir: string
  files: Set<string>
  srcDir: string
}

const testFileRegex = /\.(?:test|spec)\.[mc]?[jt]sx?$/i
const routeFileRegex =
  /(^|\/)(page|layout|template|default|loading|error|not-found|global-error)\.[^./]+$/
const leafRouteFileRegex = /(^|\/)page\.[^./]+$/

let cachedOxcParser: false | OxcParser | undefined

/**
 * The static import graph of every source file under `srcDir` — the shared
 * input of `buildCanonicalMap`, `computeFileRoutes` and `computeFileReach`.
 * Build it once with `buildStaticImportGraph` and pass it to each builder via
 * their optional `graph` field so a plugin enabling both collapse and atom
 * hoisting reads and parses every source file only ONCE per build.
 */
export interface StaticImportGraph {
  readonly requests?: readonly ImportGraphRequest[]
  /** absolute paths, sorted by POSIX path relative to `srcDir`. */
  files: string[]
  fileSet: Set<string>
  /** file -> statically imported files (within `srcDir`). */
  staticImports: Map<string, Set<string>>
  /** file -> files statically importing it (within `srcDir`). */
  staticImporters: Map<string, Set<string>>
  /** files loaded via dynamic `import()` somewhere under `srcDir`. */
  dynamicTargets: Set<string>
  /**
   * file -> files it loads via dynamic `import()` (within `srcDir`). The
   * per-importer counterpart of `dynamicTargets`: reachability needs to know
   * WHICH file lazy-loads what, otherwise a dynamic target whose only importer
   * is itself unreachable would be treated as compiled.
   */
  dynamicImports: Map<string, Set<string>>
  /**
   * file -> bare runtime import specifiers that resolve outside `srcDir`.
   * Bundler plugins use this to prewarm explicitly included package entries
   * (for example `@devup-ui/reset-css`) before their first CSS snapshot.
   */
  externalImports?: Map<string, Set<string>>
}

export interface ScannedStaticImportGraph extends StaticImportGraph {
  readonly requests: readonly ImportGraphRequest[]
}

/**
 * Scan `srcDir` once (`listSourceFiles` -> read -> parse imports -> resolve)
 * and return the full static import graph. All three graph consumers accept
 * the result, so the file I/O and parsing cost is paid a single time.
 */
export function buildStaticImportGraph(
  srcDir: string | string[],
  tsconfigPath: string | undefined,
  options: PreparedGraphOptions,
): Promise<ScannedStaticImportGraph>
export function buildStaticImportGraph(
  srcDir: string | string[],
  tsconfigPath?: string,
  options?: SyncGraphOptions,
): ScannedStaticImportGraph
export function buildStaticImportGraph(
  srcDir: string | string[],
  tsconfigPath?: string,
  options: SyncGraphOptions | PreparedGraphOptions = {},
): ScannedStaticImportGraph | Promise<ScannedStaticImportGraph> {
  if (options.prepareSource)
    return drivePreparedGraph(srcDir, tsconfigPath, options)
  const traversal = traverseGraph(srcDir, tsconfigPath, options)
  let step = traversal.next()
  while (!step.done) step = traversal.next(undefined)
  return step.value
}

export type PreparedSource =
  | string
  | {
      readonly code: string
      readonly map?: unknown
      readonly sourceType?: import('./prepared-source').SourceType
    }
  | undefined
export type PrepareSource = (
  filename: string,
) => PreparedSource | Promise<PreparedSource>
export interface SyncGraphOptions extends StaticImportGraphOptions {
  readonly prepareSource?: never
}
export interface PreparedGraphOptions extends StaticImportGraphOptions {
  readonly prepareSource: PrepareSource
}

class GraphPreparationError extends Error {
  constructor(filename: string, cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause)
    const location =
      cause instanceof Error &&
      !(cause instanceof PreparedSourceTypeError) &&
      'line' in cause &&
      typeof cause.line === 'number'
        ? `${cause.line}:${'column' in cause && typeof cause.column === 'number' ? cause.column : 1}`
        : '1:1'
    super(
      `${filename}:${location}: Graph source preparation failed: ${message}`,
      { cause },
    )
    this.name = 'GraphPreparationError'
  }
}

async function drivePreparedGraph(
  srcDir: string | string[],
  tsconfigPath: string | undefined,
  options: PreparedGraphOptions,
): Promise<ScannedStaticImportGraph> {
  const traversal = traverseGraph(srcDir, tsconfigPath, options)
  let step = traversal.next()
  while (!step.done) {
    const filename = step.value
    let prepared: PreparedSource
    try {
      prepared = readPreparedSource(await options.prepareSource(filename))
    } catch (cause) {
      const located = new GraphPreparationError(filename, cause)
      if (cause instanceof PreparedSourceTypeError) throw located
      const remapped = remapMdxError(located, filename)
      throw new Error(
        remapped.message.replaceAll(
          '(in compiled MDX)',
          '(in compiled output)',
        ),
        { cause },
      )
    }
    step = traversal.next(prepared)
  }
  return step.value
}

function* traverseGraph(
  srcDir: string | string[],
  tsconfigPath: string | undefined,
  options: StaticImportGraphOptions,
): Generator<string, ScannedStaticImportGraph, PreparedSource> {
  const cwd = resolve(options.cwd ?? process.cwd())
  const roots = (typeof srcDir === 'string' ? [srcDir] : srcDir).map((dir) =>
    resolve(cwd, dir),
  )
  const files = [
    ...new Set(
      roots.flatMap((root) => listSourceFiles(root, options.exclude, options)),
    ),
  ].sort(compareCodePoints)
  const fileSet = new Set(files)
  const directoryExclusion = createDirectoryExclusion(options.exclude)
  const exclusionObservation: { outcome: ImportRequestOutcome } = {
    outcome: { kind: 'unresolved' },
  }
  const excludedDirectory = (directory: string) => {
    const entry = directoryExclusion.match(directory)
    if (entry !== undefined)
      exclusionObservation.outcome = { kind: 'excluded', entry }
    return entry !== undefined
  }
  let resolutionInputs = createResolutionInputs().snapshot()
  const graphInputs = createResolutionInputs()
  const resolver = createModulePathResolver(
    {
      cwd,
      tsconfigPath: tsconfigPath ?? join(cwd, 'tsconfig.json'),
      conditions: options.conditions,
      alias: options.alias,
      includeMdx: options.includeMdx,
      onResolutionInputs: (inputs) => {
        resolutionInputs = inputs
        options.onResolutionInputs?.(inputs)
      },
    },
    excludedDirectory,
  )
  const excluded = createNodeModulesExcludeRegex(options.include ?? [])
  const includedRoots = new Set<string>()
  const localAliasRoots = new Set<string>()
  const staticImporters = new Map<string, Set<string>>()
  const staticImports = new Map<string, Set<string>>()
  const dynamicImports = new Map<string, Set<string>>()
  const dynamicTargets = new Set<string>()
  const externalImports = new Map<string, Set<string>>()
  const requests: ImportGraphRequest[] = []

  for (const file of files) {
    staticImporters.set(file, new Set())
    staticImports.set(file, new Set())
    dynamicImports.set(file, new Set())
    externalImports.set(file, new Set())
  }

  for (let index = 0; index < files.length; index += 1) {
    const file = files[index]
    const prepared = yield file
    graphInputs.file(file)
    options.onResolutionInputs?.(graphInputs.snapshot())
    const code = typeof prepared === 'string' ? prepared : prepared?.code
    const imports =
      code === undefined
        ? parseImports(file, readFileSync(file, 'utf-8'))
        : parsePreparedImports(
            file,
            code,
            typeof prepared === 'object' ? prepared.map : undefined,
            typeof prepared === 'object' ? prepared.sourceType : undefined,
          )
    for (const importRef of imports) {
      const request = {
        importer: file,
        request: importRef.request,
        specifier: importRef.specifier,
        kind: importRef.requestKind,
        position: importRef.position,
        source: code === undefined ? 'source' : 'compiled',
        ...(code === undefined
          ? {}
          : { map: typeof prepared === 'object' ? prepared.map : undefined }),
      } satisfies Omit<ImportGraphRequest, 'outcome'>
      exclusionObservation.outcome = { kind: 'unresolved' }
      try {
        const resolution = resolver(importRef.specifier, file)
        if (resolution === false) {
          requests.push({ ...request, outcome: exclusionObservation.outcome })
          continue
        }
        if (resolution?.ignored === true) {
          requests.push({ ...request, outcome: { kind: 'ignored' } })
          continue
        }
        const resolved = resolution?.path
        if (resolved && excludedDirectory(dirname(resolved))) {
          requests.push({ ...request, outcome: exclusionObservation.outcome })
          continue
        }
        requests.push({
          ...request,
          outcome:
            resolved !== undefined
              ? { kind: 'resolved', path: resolved, inputs: resolutionInputs }
              : !importRef.specifier.startsWith('.') &&
                  !importRef.specifier.startsWith('/') &&
                  !isAbsolute(importRef.specifier)
                ? { kind: 'external', request: importRef.specifier }
                : { kind: 'unresolved' },
        })
        const rewritten = resolution?.request ?? importRef.specifier
        const local =
          resolved !== undefined &&
          isInsideDir(cwd, resolved) &&
          !relative(cwd, resolved).split(/[\\/]/).includes('node_modules')
        if (resolved && local && rewritten !== importRef.specifier)
          localAliasRoots.add(dirname(resolved))
        if (
          resolved &&
          !importRef.specifier.startsWith('.') &&
          !isAbsolute(importRef.specifier)
        ) {
          const request = rewritten
          const parts = request.split('/')
          const name = parts.slice(0, request.startsWith('@') ? 2 : 1).join('/')
          if (!excluded.test(`node_modules/${name}/`)) {
            const dir = findPackage(dirname(file), name, undefined, graphInputs)
            if (dir) includedRoots.add(realpathSync(dir))
          }
          if (isAbsolute(request) && !local && !excluded.test(resolved)) {
            let directory = dirname(resolved)
            while (
              dirname(directory) !== directory &&
              !isFile(join(directory, 'package.json'), graphInputs)
            )
              directory = dirname(directory)
            if (isFile(join(directory, 'package.json'), graphInputs))
              includedRoots.add(realpathSync(directory))
          }
        }
        const target =
          resolved &&
          isSelectedSource(resolved, options.includeMdx) &&
          (fileSet.has(resolved) ||
            roots.some(
              (root) =>
                isInsideDir(root, resolved) &&
                !relative(root, resolved)
                  .split(/[\\/]/)
                  .includes('node_modules'),
            ) ||
            (local &&
              [...localAliasRoots].some((root) =>
                isInsideDir(root, resolved),
              )) ||
            (!excluded.test(resolved) &&
              [...includedRoots].some((root) => isInsideDir(root, resolved))))
            ? resolved
            : undefined
        if (!target) {
          if (
            !importRef.specifier.startsWith('.') &&
            !importRef.specifier.startsWith('/') &&
            !isAbsolute(importRef.specifier)
          ) {
            externalImports.get(file)?.add(importRef.specifier)
          }
          continue
        }
        if (!fileSet.has(target)) {
          fileSet.add(target)
          files.push(target)
          staticImporters.set(target, new Set())
          staticImports.set(target, new Set())
          dynamicImports.set(target, new Set())
          externalImports.set(target, new Set())
        }
        if (importRef.kind === 'dynamic') {
          dynamicTargets.add(target)
          dynamicImports.get(file)?.add(target)
          continue
        }
        staticImporters.get(target)?.add(file)
        staticImports.get(file)?.add(target)
      } catch (error) {
        retainImportGraphFailure(error, {
          ...request,
          outcome: { kind: 'error', error },
        })
        throw error
      } finally {
        options.onResolutionInputs?.(graphInputs.snapshot())
      }
    }
  }

  return {
    requests: freezeImportRequests(requests),
    files: files.sort(compareCodePoints),
    fileSet,
    staticImports,
    staticImporters,
    dynamicTargets,
    dynamicImports,
    externalImports,
  }
}

export interface StaticImportGraphOptions {
  readonly onResolutionInputs?: ResolutionInputObserver
  /** Include MDX only when the caller compiles it before extraction. */
  readonly includeMdx?: MdxSelection
  readonly alias?: ModuleAliasOptions
  readonly cwd?: string
  readonly include?: readonly string[]
  readonly conditions?: readonly string[]
  /** Bare directory names at any depth, or absolute directories and descendants. */
  readonly exclude?: readonly string[]
}

function graphRoot(srcDir: string | string[], cwd: string): string {
  if (typeof srcDir === 'string') return resolve(cwd, srcDir)
  let root = resolve(cwd, srcDir[0] ?? '.')
  for (const dir of srcDir) {
    const absolute = resolve(cwd, dir)
    while (!isInsideDir(root, absolute) && dirname(root) !== root)
      root = dirname(root)
  }
  return root
}

export function buildCanonicalMap(
  opts: BuildCanonicalMapOptions,
): Record<string, string> {
  const cwd = resolve(opts.cwd)
  const srcDir = graphRoot(opts.srcDir, cwd)
  const { files, staticImports, staticImporters, dynamicTargets } =
    opts.graph ??
    buildStaticImportGraph(opts.srcDir, opts.tsconfigPath, { cwd })

  const globalFiles = getRouteReachableGlobalFiles(
    files,
    srcDir,
    staticImports,
    opts.hoistV,
  )

  const roots = new Set<string>()
  for (const file of files) {
    const relPath = toPosixRelative(srcDir, file)
    const importerCount = staticImporters.get(file)?.size ?? 0
    if (
      routeFileRegex.test(relPath) ||
      importerCount !== 1 ||
      dynamicTargets.has(file)
    ) {
      roots.add(file)
    }
  }

  for (const cycleRoot of findClosedCycles(files, roots, staticImporters)) {
    roots.add(cycleRoot)
  }

  const parents = buildParentsMap(files, roots, staticImporters)

  const toKey = makeToKey(cwd, opts.keyBy ?? 'cwd-relative')
  const map: Record<string, string> = {}
  for (const file of files) {
    if (globalFiles.has(file)) {
      map[toKey(file)] = '@global'
      continue
    }
    if (roots.has(file)) continue
    const bucketRoot = findBucketRoot(file, parents, roots)
    if (bucketRoot === file) continue
    map[toKey(file)] = toKey(bucketRoot)
  }

  return map
}

export interface ComputeFileRoutesOptions {
  srcDir: string | string[]
  tsconfigPath?: string
  cwd: string
  /** pre-built graph from `buildStaticImportGraph` to skip the file scan. */
  graph?: StaticImportGraph
}

/**
 * Map every source file to the set of leaf-route ids whose render closure
 * includes it. This is the input the atom-level hoisting engine needs
 * (`importFileRoutes`): an atom used by `>= threshold` distinct routes is
 * hoisted into the shared `devup-ui.css`, the rest stay in per-route chunks.
 *
 * Keys are POSIX paths relative to `cwd` (the same convention as
 * `buildCanonicalMap`, which matches the extraction filename the loader passes).
 * Route ids are assigned by sorted leaf-route order, so they are stable across
 * runs. A file reachable from no leaf route is omitted (it contributes no route
 * count and therefore never hoists on its own).
 */
export function computeFileRoutes(
  opts: ComputeFileRoutesOptions,
): Record<string, number[]> {
  const cwd = resolve(opts.cwd)
  const srcDir = graphRoot(opts.srcDir, cwd)
  const { files, staticImports } =
    opts.graph ??
    buildStaticImportGraph(opts.srcDir, opts.tsconfigPath, { cwd })

  const leafRoutes = files
    .filter((file) => leafRouteFileRegex.test(toPosixRelative(srcDir, file)))
    .sort((a, b) =>
      compareCodePoints(toPosixRelative(srcDir, a), toPosixRelative(srcDir, b)),
    )
  const routeShellFilesByDir = getRouteShellFilesByDir(files, srcDir)

  const fileRoutes: Record<string, number[]> = {}
  leafRoutes.forEach((leafRoute, routeId) => {
    const closure = getLeafRouteClosure(
      leafRoute,
      srcDir,
      staticImports,
      routeShellFilesByDir,
    )
    for (const file of closure) {
      const key = toPosixRelative(cwd, file)
      ;(fileRoutes[key] ??= []).push(routeId)
    }
  })

  return fileRoutes
}

export interface ComputeCompiledFilesOptions {
  srcDir: string | string[]
  tsconfigPath?: string
  cwd: string
  /** pre-built graph from `buildStaticImportGraph` to skip the file scan. */
  graph?: StaticImportGraph
}

/**
 * Every source file the bundler will compile for the app's routes: the closure
 * of all leaf routes (plus their ancestor route shells) over BOTH static and
 * dynamic `import()` edges.
 *
 * This is deliberately WIDER than `computeFileRoutes`, whose closure stops at
 * static edges because a lazily-loaded file belongs to its own chunk for
 * hoisting purposes. Completion tracking needs the opposite guarantee: a file
 * behind `dynamic(() => import(...))` is still compiled, still POSTs
 * `/extract`, and still contributes atoms to the shared sheet — so a base-CSS
 * wait built from the static-only set resolves BEFORE those atoms exist and
 * serves a sheet missing every lazily-loaded component's styles.
 *
 * Keys are POSIX paths relative to `cwd`, matching the extraction filename the
 * loader posts. Returns a sorted array (deterministic across runs). Empty when
 * no leaf route is detected, which keeps callers on their idle fallback rather
 * than blocking on a set that can never complete.
 */
export function computeCompiledFiles(
  opts: ComputeCompiledFilesOptions,
): string[] {
  const cwd = resolve(opts.cwd)
  const srcDir = graphRoot(opts.srcDir, cwd)
  const { files, staticImports, dynamicImports } =
    opts.graph ??
    buildStaticImportGraph(opts.srcDir, opts.tsconfigPath, { cwd })

  const allImports = new Map<string, Set<string>>()
  for (const file of files) {
    const edges = new Set(staticImports.get(file))
    for (const target of dynamicImports.get(file) ?? []) edges.add(target)
    allImports.set(file, edges)
  }

  const routeShellFilesByDir = getRouteShellFilesByDir(files, srcDir)
  const compiled = new Set<string>()
  for (const file of files) {
    if (!leafRouteFileRegex.test(toPosixRelative(srcDir, file))) continue
    for (const reached of getLeafRouteClosure(
      file,
      srcDir,
      allImports,
      routeShellFilesByDir,
    )) {
      compiled.add(reached)
    }
  }

  return [...compiled].map((file) => toPosixRelative(cwd, file)).sort()
}

export interface ComputeReachableFilesOptions {
  srcDir: string | string[]
  tsconfigPath?: string
  /** The bundler's entry modules, as absolute paths with or without extension. */
  entries: string[]
  /** pre-built graph from `buildStaticImportGraph` to skip the file scan. */
  graph?: StaticImportGraph
}

/**
 * The source files under `srcDir` a bundler compiles from `entries`: their
 * closure over static and dynamic `import()` edges, as absolute paths in the
 * graph's order. Extracting them before bundling fills the shared stylesheet
 * without the styles of files no entry imports.
 */
export function computeReachableFiles(
  opts: ComputeReachableFilesOptions,
): string[] {
  const { files, fileSet, staticImports, dynamicImports } =
    opts.graph ?? buildStaticImportGraph(opts.srcDir, opts.tsconfigPath)
  const queue = opts.entries
    .map((entry) => resolveFile(resolve(entry)))
    .filter(
      (entry): entry is string =>
        typeof entry === 'string' && fileSet.has(entry),
    )
  const reached = new Set<string>()
  for (let index = 0; index < queue.length; index += 1) {
    const file = queue[index]
    if (reached.has(file)) continue
    reached.add(file)
    for (const imports of [staticImports, dynamicImports]) {
      for (const target of imports.get(file) ?? []) queue.push(target)
    }
  }
  return files.filter((file) => reached.has(file))
}

export interface ComputeFileReachOptions {
  srcDir: string | string[]
  tsconfigPath?: string
  cwd: string
  /**
   * Optional explicit entry files (absolute or `cwd`-relative). When provided,
   * these override the default heuristic. Use this when the bundler knows its
   * real entry points (e.g. `rollupOptions.input`); otherwise the heuristic
   * (files with no importer within `srcDir`, plus dynamic-import targets) is
   * used as a fallback.
   */
  entries?: string[]
  keyBy?: GraphKeyMode
  /** pre-built graph from `buildStaticImportGraph` to skip the file scan. */
  graph?: StaticImportGraph
}

/**
 * Bundler-agnostic generalization of `computeFileRoutes`: map every source file
 * to the set of ENTRY ids whose static import closure includes it.
 *
 * "Entries" are the independently-loaded boundaries: files with no importer
 * within `srcDir` plus dynamic-import targets, OR an explicit `entries`
 * override. This is the importer-graph signal that replaces Next's route
 * concept, so atom hoisting works for any bundler.
 *
 * Keys are POSIX paths relative to `cwd` (matching the extraction filename and
 * `buildCanonicalMap` keys). Entry ids are assigned by sorted entry order
 * (stable). A file reached by no entry is omitted. A single-entry app yields
 * reach 1 for everything, so nothing hoists — correct, since one bucket is
 * already optimal there.
 */
export function computeFileReach(
  opts: ComputeFileReachOptions,
): Record<string, number[]> {
  const cwd = resolve(opts.cwd)
  const srcDir = graphRoot(opts.srcDir, cwd)
  const { files, fileSet, staticImports, staticImporters, dynamicTargets } =
    opts.graph ??
    buildStaticImportGraph(opts.srcDir, opts.tsconfigPath, { cwd })

  let entries: string[]
  if (opts.entries && opts.entries.length > 0) {
    entries = opts.entries
      .map((entry) => resolve(cwd, entry))
      .filter((entry) => fileSet.has(entry))
  } else {
    entries = files.filter(
      (file) =>
        (staticImporters.get(file)?.size ?? 0) === 0 ||
        dynamicTargets.has(file),
    )
  }
  entries = [...new Set(entries)].sort((a, b) =>
    compareCodePoints(toPosixRelative(srcDir, a), toPosixRelative(srcDir, b)),
  )

  const toKey = makeToKey(cwd, opts.keyBy ?? 'cwd-relative')
  const fileReach: Record<string, number[]> = {}
  entries.forEach((entry, entryId) => {
    for (const file of getStaticClosure(entry, staticImports)) {
      const key = toKey(file)
      ;(fileReach[key] ??= []).push(entryId)
    }
  })

  return fileReach
}

export interface AtomHoistPlan {
  /** atom-hoist threshold to pass to setAtomHoist (clamped to >= 2). */
  threshold: number
  /** canonical bucket -> route ids reaching it (input to importFileRoutes). */
  reachByBucket: Record<string, number[]>
}

/**
 * Shared fold + gate + clamp for atom-level hoisting, used identically by every
 * bundler plugin (next/vite/webpack/rsbuild). Given the canonical (collapse) map
 * and a file -> route-ids reach map, it folds reach onto the canonical bucket
 * (the engine keys property buckets by `canonical(filename)`), skips the
 * `@global` bucket, and returns the hoist plan — or `null` when fewer than two
 * distinct routes exist (atom hoisting is then a no-op; a single bucket is
 * already optimal).
 *
 * Extracting this removes a subtle, error-prone block (fold / `@global` skip /
 * id dedupe / `>= 2` gate / `max(2, n)` clamp) from four plugin copies into one
 * tested place.
 */
export function planAtomHoist(
  canonicalMap: Record<string, string>,
  fileReach: Record<string, number[]>,
  atomHoist: number,
): AtomHoistPlan | null {
  const reachByBucket: Record<string, number[]> = {}
  for (const [file, ids] of Object.entries(fileReach)) {
    const bucket = canonicalMap[file] ?? file
    if (bucket === '@global') continue
    const set = (reachByBucket[bucket] ??= [])
    for (const id of ids) if (!set.includes(id)) set.push(id)
  }
  const routeCount = new Set(Object.values(fileReach).flat()).size
  if (routeCount < 2) return null
  return { threshold: Math.max(2, atomHoist), reachByBucket }
}

function getRouteReachableGlobalFiles(
  files: string[],
  srcDir: string,
  staticImports: Map<string, Set<string>>,
  hoistV: number | undefined,
): Set<string> {
  if (hoistV === undefined || hoistV <= 0) return new Set()

  const leafRoutes = files.filter((file) =>
    leafRouteFileRegex.test(toPosixRelative(srcDir, file)),
  )
  const routeShellFilesByDir = getRouteShellFilesByDir(files, srcDir)
  const threshold = leafRoutes.length / hoistV
  const reachedBy = new Map<string, number>()

  for (const leafRoute of leafRoutes) {
    const closure = getLeafRouteClosure(
      leafRoute,
      srcDir,
      staticImports,
      routeShellFilesByDir,
    )
    for (const file of closure) {
      reachedBy.set(file, (reachedBy.get(file) ?? 0) + 1)
    }
  }

  const globalFiles = new Set<string>()
  for (const [file, routeCount] of reachedBy) {
    if (routeCount >= threshold && routeCount >= 2) {
      globalFiles.add(file)
    }
  }

  return globalFiles
}

function getRouteShellFilesByDir(
  files: string[],
  srcDir: string,
): Map<string, string[]> {
  const routeShellFilesByDir = new Map<string, string[]>()

  for (const file of files) {
    const relPath = toPosixRelative(srcDir, file)
    if (!routeFileRegex.test(relPath) || leafRouteFileRegex.test(relPath)) {
      continue
    }

    const dir = dirname(file)
    const routeShellFiles = routeShellFilesByDir.get(dir) ?? []
    routeShellFiles.push(file)
    routeShellFilesByDir.set(dir, routeShellFiles)
  }

  return routeShellFilesByDir
}

function getLeafRouteClosure(
  leafRoute: string,
  srcDir: string,
  staticImports: Map<string, Set<string>>,
  routeShellFilesByDir: Map<string, string[]>,
): Set<string> {
  const closure = getStaticClosure(leafRoute, staticImports)

  for (const routeShellFile of getAncestorRouteShellFiles(
    leafRoute,
    srcDir,
    routeShellFilesByDir,
  )) {
    for (const file of getStaticClosure(routeShellFile, staticImports)) {
      closure.add(file)
    }
  }

  return closure
}

function getAncestorRouteShellFiles(
  leafRoute: string,
  srcDir: string,
  routeShellFilesByDir: Map<string, string[]>,
): string[] {
  const routeShellFiles: string[] = []
  let currentDir = dirname(leafRoute)

  while (isInsideDir(srcDir, currentDir)) {
    const currentRouteShellFiles = routeShellFilesByDir.get(currentDir)
    if (currentRouteShellFiles) routeShellFiles.push(...currentRouteShellFiles)
    if (currentDir === srcDir) break
    const parentDir = dirname(currentDir)
    if (parentDir === currentDir) break
    currentDir = parentDir
  }

  return routeShellFiles
}

function getStaticClosure(
  routeEntry: string,
  staticImports: Map<string, Set<string>>,
): Set<string> {
  const closure = new Set<string>()
  const queue = [routeEntry]

  for (let index = 0; index < queue.length; index += 1) {
    const file = queue[index]
    if (closure.has(file)) continue
    closure.add(file)

    const importedFiles = staticImports.get(file)
    if (!importedFiles) continue
    for (const importedFile of importedFiles) {
      if (!closure.has(importedFile)) queue.push(importedFile)
    }
  }

  return closure
}

/** Order that is the same on every machine, unlike localeCompare. */
export function compareCodePoints(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0
}

/**
 * Enumerate every extractable source file under `srcDir`, sorted by POSIX path
 * (deterministic order). Skips `node_modules`, test/spec files, and non-JS/TS
 * files — the SAME filter `buildCanonicalMap` uses internally, so a plugin can
 * pre-warm the extractor over exactly the file set the canonical map was built
 * from. Returns absolute paths.
 */
export function listSourceFiles(
  srcDir: string,
  exclude: readonly string[] = [],
  options: SourceSelectionOptions = {},
): string[] {
  const files: string[] = []
  const excludedDirectory = createDirectoryExclusion(exclude)

  function visit(dir: string): void {
    if (excludedDirectory(resolve(dir))) return
    if (!existsSync(dir)) return
    const entries = readdirSync(dir, { withFileTypes: true }).sort((a, b) =>
      compareCodePoints(a.name, b.name),
    )
    for (const entry of entries) {
      const entryPath = join(dir, entry.name)
      if (entry.isDirectory()) {
        if (entry.name === 'node_modules') continue
        visit(entryPath)
        continue
      }
      if (!entry.isFile()) continue
      if (!isSelectedSource(entry.name, options.includeMdx)) continue
      if (testFileRegex.test(entry.name)) continue
      files.push(resolve(entryPath))
    }
  }

  visit(srcDir)
  return files.sort((a, b) =>
    compareCodePoints(toPosixRelative(srcDir, a), toPosixRelative(srcDir, b)),
  )
}

function parseImports(
  filename: string,
  source: string,
): ImportRequestReference[] {
  if (!SOURCE_FILE_RE.test(filename))
    return scanImportRequests(mdxEsmSource(source), false, false)
  return parseSourceImports(filename, source)
}

function parsePreparedImports(
  filename: string,
  source: string,
  map: unknown,
  sourceType?: import('./prepared-source').SourceType,
): ImportRequestReference[] {
  return parseSourceImports(filename, source, { map, sourceType })
}

function parseSourceImports(
  filename: string,
  source: string,
  prepared?: {
    readonly map: unknown
    readonly sourceType?: import('./prepared-source').SourceType
  },
): ImportRequestReference[] {
  const parser = getOxcParser()
  try {
    const ast = parser?.parseSync(
      filename,
      source,
      prepared &&
        (prepared.sourceType === 'compiled-mdx' ||
          !/\.[mc]?tsx?$/i.test(filename))
        ? { sourceType: 'module', lang: 'jsx' }
        : { sourceType: 'module' },
    )
    if (isRecord(ast) && Array.isArray(ast.errors) && ast.errors.length) {
      throw new Error(preparedDiagnostics(filename, source, ast.errors))
    }
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause)
    const located = message.startsWith(`${filename}:`)
      ? cause
      : new GraphPreparationError(filename, cause)
    if (!prepared) throw located
    const remapped = remapMdxError(located, filename, prepared.map)
    throw new Error(
      remapped.message.replaceAll('(in compiled MDX)', '(in compiled output)'),
      { cause },
    )
  }
  return scanImportRequests(
    source,
    prepared?.sourceType === 'compiled-mdx' ||
      /\.[jt]sx$/i.test(filename) ||
      (prepared !== undefined && !/\.[mc]?ts$/i.test(filename)),
    prepared?.sourceType !== 'compiled-mdx' && /\.[mc]?tsx?$/i.test(filename),
  )
}

function mdxEsmSource(source: string): string {
  const blocks: string[] = []
  let fence = ''
  let esm = false
  let comment = false
  for (const line of source.split(/(?<=\n)/)) {
    const blank = line.replace(/[^\r\n]/g, ' ')
    if (line.includes('<!--')) comment = true
    if (comment) {
      if (line.includes('-->')) comment = false
      esm = false
      blocks.push(blank)
      continue
    }
    const marker = /^\s{0,3}(`{3,}|~{3,})/.exec(line)?.[1]
    if (marker) {
      if (!fence) fence = marker
      else if (marker[0] === fence[0] && marker.length >= fence.length)
        fence = ''
      esm = false
      blocks.push(blank)
      continue
    }
    if (fence) {
      blocks.push(blank)
      continue
    }
    if (!line.trim()) {
      esm = false
      blocks.push(blank)
      continue
    }
    if (/^(?:import\s+(?!\()|export\s+)/.test(line)) esm = true
    blocks.push(esm ? line : blank)
  }
  return blocks.join('')
}

function getOxcParser(): OxcParser | undefined {
  if (cachedOxcParser !== undefined) {
    return cachedOxcParser || undefined
  }

  try {
    const require = createRequire(import.meta.url)
    const parser = require('oxc-parser') as Partial<OxcParser>
    cachedOxcParser =
      typeof parser.parseSync === 'function' ? (parser as OxcParser) : false
  } catch {
    cachedOxcParser = false
  }

  return cachedOxcParser || undefined
}

/**
 * @internal test-only: force the cached oxc parser. oxc-parser is an optional
 * peer that is absent in this repo, so the diagnostic path is otherwise unreachable
 * from tests; module state is shared across test files (no per-file reset), so
 * `mock.module` cannot toggle it deterministically. Pass `undefined` to clear
 * the cache and re-detect. The scanner remains the sole edge authority.
 */
export function __setOxcParserForTest(
  parser: OxcParser | false | undefined,
): void {
  cachedOxcParser = parser
}

/** A module an import resolved to, as `setModuleResolver` expects it. */
export interface ResolvedModule {
  readonly ignored?: never
  path: string
  code: string
  readonly sourceType?: import('./prepared-source').SourceType
}

/** Successful native ignore: no filesystem module or manufactured source. */
export interface IgnoredModule {
  readonly ignored: true
  readonly path?: never
  readonly code?: never
  readonly sourceType?: never
  readonly request?: never
}

export type ModuleResolution = ResolvedModule | IgnoredModule

export interface CreateModuleResolverOptions {
  readonly onResolutionInputs?: ResolutionInputObserver
  readonly prepareSource?: PrepareSource
  readonly alias?: ModuleAliasOptions
  readonly includeMdx?: MdxSelection
  cwd?: string
  tsconfigPath?: string
  conditions?: readonly string[]
  /**
   * The name the plugin extracts a file under, given its absolute path. A
   * resolved module is extracted under this name, so it has to be the one the
   * bundler later extracts the same file under.
   */
  toId?: (path: string) => string
}

/**
 * Resolve the imports of extracted files like the bundler: relative paths,
 * tsconfig `paths`, then packages in `node_modules` (`exports` conditions,
 * `module`, `main`), reading each module.
 */
export function createModuleResolver({
  toId = (path) => path,
  ...options
}: CreateModuleResolverOptions = {}): ModuleResolver {
  const resolver = createModulePathResolver({
    ...options,
    includeMdx: options.includeMdx ?? true,
  })
  const prepared = createPreparedResolver(options)
  return Object.assign(
    (specifier: string, importer: string) => {
      const resolution = resolver(specifier, importer)
      if (!resolution) return undefined
      if (resolution.ignored === true) return resolution
      const { path } = resolution
      const source = prepared.read(
        path,
        resolve(options.cwd ?? process.cwd(), importer),
      )
      return prepared.resolved(toId(path), path, source)
    },
    { remapError: prepared.remapError },
  )
}

export type ModulePathResolverOptions = Pick<
  CreateModuleResolverOptions,
  | 'cwd'
  | 'tsconfigPath'
  | 'conditions'
  | 'alias'
  | 'includeMdx'
  | 'onResolutionInputs'
>
export type ModulePathResolution =
  AliasResolution | IgnoredModule | false | undefined
export type ModulePathResolver = (
  specifier: string,
  importer: string,
) => ModulePathResolution

export function createModulePathResolver(
  {
    cwd = process.cwd(),
    tsconfigPath = join(cwd, 'tsconfig.json'),
    conditions = ['import', 'module', 'require', 'node'],
    alias = {},
    includeMdx,
    onResolutionInputs,
  }: ModulePathResolverOptions = {},
  excludedDirectory: (
    directory: string,
  ) => boolean = createDirectoryExclusion(),
): ModulePathResolver {
  const setupInputs = createResolutionInputs()
  const config = (() => {
    try {
      return readPathAliases(tsconfigPath, setupInputs)
    } finally {
      onResolutionInputs?.(setupInputs.snapshot())
    }
  })()
  const { aliases, baseDir, baseUrl } = config
  const setup = setupInputs.snapshot()
  const extensions = sourceExtensions(includeMdx)
  return (specifier, importer) => {
    const inputs = createResolutionInputs(setup)
    const fileResolver = (path: string) =>
      resolveFile(path, { extensions, excludedDirectory, inputs })
    const from = resolve(cwd, importer)
    try {
      return resolveModuleAlias(specifier, {
        alias,
        importer: from,
        resolveRequest: (request, aliased) =>
          request.startsWith('.')
            ? fileResolver(resolve(dirname(from), request))
            : isAbsolute(request)
              ? fileResolver(request)
              : (resolveAliasCandidates(request, {
                  aliases,
                  aliasBaseDir: baseDir,
                })
                  .map(fileResolver)
                  .find((candidate) => candidate !== undefined) ??
                (baseUrl
                  ? fileResolver(resolve(baseUrl, request))
                  : undefined) ??
                resolvePackage(request, from, {
                  conditions,
                  fileResolver,
                  excludedDirectory,
                  aliased,
                  inputs,
                })),
      })
    } finally {
      onResolutionInputs?.(inputs.snapshot())
    }
  }
}

function resolveAliasCandidates(
  specifier: string,
  context: Pick<ResolveContext, 'aliases' | 'aliasBaseDir'>,
): string[] {
  const candidates: string[] = []
  for (const alias of context.aliases) {
    if (
      !specifier.startsWith(alias.prefix) ||
      !specifier.endsWith(alias.suffix) ||
      (alias.exact && specifier !== alias.prefix) ||
      specifier.length < alias.prefix.length + alias.suffix.length
    ) {
      continue
    }
    const matched = specifier.slice(
      alias.prefix.length,
      specifier.length - alias.suffix.length,
    )
    for (const target of alias.targets) {
      candidates.push(
        resolve(context.aliasBaseDir, target.replace('*', matched)),
      )
    }
    break
  }
  return candidates
}

function isInsideDir(dir: string, file: string): boolean {
  const relPath = relative(dir, file)
  return relPath === '' || (!relPath.startsWith('..') && !isAbsolute(relPath))
}

function buildParentsMap(
  files: string[],
  roots: Set<string>,
  staticImporters: Map<string, Set<string>>,
): Map<string, string> {
  const parents = new Map<string, string>()
  for (const file of files) {
    if (roots.has(file)) continue
    const importers = staticImporters.get(file)
    if (importers?.size !== 1) continue
    const [importer] = importers
    parents.set(file, importer)
  }
  return parents
}

function findClosedCycles(
  files: string[],
  roots: Set<string>,
  staticImporters: Map<string, Set<string>>,
): Set<string> {
  const parents = buildParentsMap(files, roots, staticImporters)

  const cycleRoots = new Set<string>()
  const visiting = new Set<string>()
  const visited = new Set<string>()
  const stack: string[] = []

  function visit(file: string): void {
    if (visited.has(file) || roots.has(file)) return
    if (visiting.has(file)) {
      const cycleStart = stack.indexOf(file)
      for (const cycleFile of stack.slice(cycleStart)) {
        cycleRoots.add(cycleFile)
      }
      return
    }

    visiting.add(file)
    stack.push(file)
    const parent = parents.get(file)
    if (parent && parents.has(parent)) visit(parent)
    stack.pop()
    visiting.delete(file)
    visited.add(file)
  }

  for (const file of files) {
    visit(file)
  }

  return cycleRoots
}

function findBucketRoot(
  file: string,
  parents: Map<string, string>,
  roots: Set<string>,
): string {
  let current = file
  const seen = new Set<string>()

  while (!roots.has(current)) {
    if (seen.has(current)) return file
    seen.add(current)
    const parent = parents.get(current)
    if (!parent) return current
    current = parent
  }

  return current
}

function toPosixRelative(from: string, to: string): string {
  return relative(from, to).replaceAll('\\', '/')
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

/** @internal CLI entry, extracted from the `import.meta.main` guard so it is
 * reachable from tests (the guard itself never runs under the test runner). */
export function runImportGraphCli(argv: string[]): void {
  const [srcDirArg, cwdArg = process.cwd(), tsconfigPathArg, outFileArg] = argv

  if (!srcDirArg) {
    console.error(
      'Usage: bun packages/next-plugin/src/import-graph.ts <srcDir> [cwd] [tsconfigPath] [outFile]',
    )
    process.exit(1)
    return
  }

  const cwd = resolve(cwdArg)
  const srcDir = resolve(cwd, srcDirArg)
  const tsconfigPath = tsconfigPathArg
    ? resolve(cwd, tsconfigPathArg)
    : undefined
  const map = buildCanonicalMap({ cwd, srcDir, tsconfigPath })
  const json = `${JSON.stringify(map, null, 2)}\n`

  if (outFileArg) {
    writeFileSync(resolve(cwd, outFileArg), json)
  } else {
    console.info(json.trimEnd())
  }
}

if (import.meta.main) runImportGraphCli(process.argv.slice(2))
