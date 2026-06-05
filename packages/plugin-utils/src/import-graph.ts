import {
  existsSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import {
  dirname,
  extname,
  isAbsolute,
  join,
  relative,
  resolve,
} from 'node:path'

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
  srcDir: string
  tsconfigPath?: string
  cwd: string
  hoistV?: number
  keyBy?: GraphKeyMode
}

interface ImportReference {
  kind: 'static' | 'dynamic'
  specifier: string
}

interface OxcParser {
  parseSync: (
    filename: string,
    source: string,
    options?: Record<string, unknown>,
  ) => unknown
}

interface PathAlias {
  prefix: string
  suffix: string
  targets: string[]
}

interface ResolveContext {
  aliases: PathAlias[]
  aliasBaseDir: string
  files: Set<string>
  srcDir: string
}

const jsExtensions = ['.ts', '.tsx', '.js', '.jsx', '.mjs']
const jsFileRegex = /\.(?:tsx?|jsx?|mjs)$/
const testFileRegex = /\.(?:test|spec)\.[mc]?[jt]sx?$/
const routeFileRegex =
  /(^|\/)(page|layout|template|default|loading|error|not-found|global-error)\.(tsx|ts|jsx|js)$/
const leafRouteFileRegex = /(^|\/)page\.(tsx|ts|jsx|js)$/

let cachedOxcParser: false | OxcParser | undefined

export function buildCanonicalMap(
  opts: BuildCanonicalMapOptions,
): Record<string, string> {
  const cwd = resolve(opts.cwd)
  const srcDir = resolve(opts.srcDir)
  const files = listSourceFiles(srcDir)
  const fileSet = new Set(files)
  const aliases = readPathAliases(opts.tsconfigPath)
  const context: ResolveContext = {
    aliases: aliases.aliases,
    aliasBaseDir: aliases.baseDir,
    files: fileSet,
    srcDir,
  }
  const staticImporters = new Map<string, Set<string>>()
  const staticImports = new Map<string, Set<string>>()
  const dynamicTargets = new Set<string>()

  for (const file of files) {
    staticImporters.set(file, new Set())
    staticImports.set(file, new Set())
  }

  for (const file of files) {
    const imports = parseImports(file, readFileSync(file, 'utf-8'))
    for (const importRef of imports) {
      const target = resolveImport(importRef.specifier, file, context)
      if (!target) continue
      if (importRef.kind === 'dynamic') {
        dynamicTargets.add(target)
        continue
      }
      staticImporters.get(target)?.add(file)
      staticImports.get(file)?.add(target)
    }
  }

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

  const parents = new Map<string, string>()
  for (const file of files) {
    if (roots.has(file)) continue
    const importers = staticImporters.get(file)
    if (importers?.size !== 1) continue
    const [importer] = importers
    parents.set(file, importer)
  }

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
  srcDir: string
  tsconfigPath?: string
  cwd: string
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
  const srcDir = resolve(opts.srcDir)
  const files = listSourceFiles(srcDir)
  const fileSet = new Set(files)
  const aliases = readPathAliases(opts.tsconfigPath)
  const context: ResolveContext = {
    aliases: aliases.aliases,
    aliasBaseDir: aliases.baseDir,
    files: fileSet,
    srcDir,
  }

  const staticImports = new Map<string, Set<string>>()
  for (const file of files) staticImports.set(file, new Set())
  for (const file of files) {
    for (const importRef of parseImports(file, readFileSync(file, 'utf-8'))) {
      if (importRef.kind !== 'static') continue
      const target = resolveImport(importRef.specifier, file, context)
      if (target) staticImports.get(file)?.add(target)
    }
  }

  const leafRoutes = files
    .filter((file) => leafRouteFileRegex.test(toPosixRelative(srcDir, file)))
    .sort((a, b) =>
      toPosixRelative(srcDir, a).localeCompare(toPosixRelative(srcDir, b)),
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

export interface ComputeFileReachOptions {
  srcDir: string
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
  const srcDir = resolve(opts.srcDir)
  const files = listSourceFiles(srcDir)
  const fileSet = new Set(files)
  const aliases = readPathAliases(opts.tsconfigPath)
  const context: ResolveContext = {
    aliases: aliases.aliases,
    aliasBaseDir: aliases.baseDir,
    files: fileSet,
    srcDir,
  }

  const staticImporters = new Map<string, Set<string>>()
  const staticImports = new Map<string, Set<string>>()
  for (const file of files) {
    staticImporters.set(file, new Set())
    staticImports.set(file, new Set())
  }
  const dynamicTargets = new Set<string>()
  for (const file of files) {
    for (const importRef of parseImports(file, readFileSync(file, 'utf-8'))) {
      const target = resolveImport(importRef.specifier, file, context)
      if (!target) continue
      if (importRef.kind === 'dynamic') {
        dynamicTargets.add(target)
        continue
      }
      staticImporters.get(target)?.add(file)
      staticImports.get(file)?.add(target)
    }
  }

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
    toPosixRelative(srcDir, a).localeCompare(toPosixRelative(srcDir, b)),
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

/**
 * Enumerate every extractable source file under `srcDir`, sorted by POSIX path
 * (deterministic order). Skips `node_modules`, test/spec files, and non-JS/TS
 * files — the SAME filter `buildCanonicalMap` uses internally, so a plugin can
 * pre-warm the extractor over exactly the file set the canonical map was built
 * from. Returns absolute paths.
 */
export function listSourceFiles(srcDir: string): string[] {
  const files: string[] = []

  function visit(dir: string): void {
    if (!existsSync(dir)) return
    const entries = readdirSync(dir, { withFileTypes: true }).sort((a, b) =>
      a.name.localeCompare(b.name),
    )
    for (const entry of entries) {
      const entryPath = join(dir, entry.name)
      if (entry.isDirectory()) {
        if (entry.name === 'node_modules') continue
        visit(entryPath)
        continue
      }
      if (!entry.isFile()) continue
      if (!jsFileRegex.test(entry.name)) continue
      if (testFileRegex.test(entry.name)) continue
      files.push(resolve(entryPath))
    }
  }

  visit(srcDir)
  return files.sort((a, b) =>
    toPosixRelative(srcDir, a).localeCompare(toPosixRelative(srcDir, b)),
  )
}

function parseImports(filename: string, source: string): ImportReference[] {
  const astImports = parseImportsWithOxc(filename, source)
  if (astImports) return astImports
  return scanImports(source)
}

function parseImportsWithOxc(
  filename: string,
  source: string,
): ImportReference[] | undefined {
  const parser = getOxcParser()
  if (!parser) return undefined

  try {
    const ast = parser.parseSync(filename, source, { sourceType: 'module' })
    const imports: ImportReference[] = []
    collectAstImports(ast, imports)
    return imports
  } catch {
    return undefined
  }
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

function collectAstImports(
  node: unknown,
  imports: ImportReference[],
  seen = new WeakSet<object>(),
): void {
  if (!isRecord(node)) return
  if (seen.has(node)) return
  seen.add(node)

  const type = typeof node.type === 'string' ? node.type : undefined
  if (
    type === 'ImportDeclaration' ||
    type === 'ExportNamedDeclaration' ||
    type === 'ExportAllDeclaration'
  ) {
    addAstImport(imports, 'static', node.source)
  } else if (type === 'ImportExpression') {
    addAstImport(imports, 'dynamic', node.source ?? node.argument)
  } else if (type === 'CallExpression' && isImportCallee(node.callee)) {
    const firstArgument = Array.isArray(node.arguments)
      ? node.arguments[0]
      : undefined
    addAstImport(imports, 'dynamic', firstArgument)
  }

  for (const value of Object.values(node)) {
    if (Array.isArray(value)) {
      for (const child of value) {
        collectAstImports(child, imports, seen)
      }
      continue
    }
    collectAstImports(value, imports, seen)
  }
}

function addAstImport(
  imports: ImportReference[],
  kind: ImportReference['kind'],
  node: unknown,
): void {
  const specifier = getStringLiteralValue(node)
  if (specifier) imports.push({ kind, specifier })
}

function getStringLiteralValue(node: unknown): string | undefined {
  if (!isRecord(node)) return undefined
  if (typeof node.value === 'string') return node.value
  if (typeof node.raw === 'string') return node.raw.slice(1, -1)
  return undefined
}

function isImportCallee(node: unknown): boolean {
  if (!isRecord(node)) return false
  return node.type === 'Import' || node.name === 'import'
}

function scanImports(source: string): ImportReference[] {
  const imports: ImportReference[] = []
  const code = stripComments(source)
  const staticImportRegex =
    /\bimport\s+(?:type\s+)?(?:[^'"`]*?\s+from\s*)?(['"])([^'"]+)\1/gm
  const exportFromRegex =
    /\bexport\s+(?:type\s+)?(?:\*[^'"`]*?|\{[^}]*\})\s+from\s*(['"])([^'"]+)\1/gm
  const dynamicImportRegex = /\bimport\s*\(\s*(['"])([^'"]+)\1\s*\)/gm

  for (const match of code.matchAll(staticImportRegex)) {
    imports.push({ kind: 'static', specifier: match[2] })
  }
  for (const match of code.matchAll(exportFromRegex)) {
    imports.push({ kind: 'static', specifier: match[2] })
  }
  for (const match of code.matchAll(dynamicImportRegex)) {
    imports.push({ kind: 'dynamic', specifier: match[2] })
  }

  return imports
}

function stripComments(source: string): string {
  let result = ''
  let index = 0
  let quote: false | '"' | "'" | '`' = false

  while (index < source.length) {
    const char = source[index]
    const next = source[index + 1]

    if (quote) {
      result += char
      if (char === '\\') {
        result += next ?? ''
        index += 2
        continue
      }
      if (char === quote) quote = false
      index += 1
      continue
    }

    if (char === '"' || char === "'" || char === '`') {
      quote = char
      result += char
      index += 1
      continue
    }

    if (char === '/' && next === '/') {
      while (index < source.length && source[index] !== '\n') {
        result += ' '
        index += 1
      }
      continue
    }

    if (char === '/' && next === '*') {
      result += '  '
      index += 2
      while (
        index < source.length &&
        !(source[index] === '*' && source[index + 1] === '/')
      ) {
        result += source[index] === '\n' ? '\n' : ' '
        index += 1
      }
      result += '  '
      index += 2
      continue
    }

    result += char
    index += 1
  }

  return result
}

function readPathAliases(tsconfigPath: string | undefined): {
  aliases: PathAlias[]
  baseDir: string
} {
  if (!tsconfigPath || !existsSync(tsconfigPath)) {
    return { aliases: [], baseDir: process.cwd() }
  }

  const configPath = resolve(tsconfigPath)
  const configDir = dirname(configPath)
  try {
    const config = JSON.parse(
      stripTrailingCommas(stripComments(readFileSync(configPath, 'utf-8'))),
    )
    if (!isRecord(config) || !isRecord(config.compilerOptions)) {
      return { aliases: [], baseDir: configDir }
    }

    const baseUrl =
      typeof config.compilerOptions.baseUrl === 'string'
        ? config.compilerOptions.baseUrl
        : '.'
    const paths = config.compilerOptions.paths
    if (!isRecord(paths)) {
      return { aliases: [], baseDir: resolve(configDir, baseUrl) }
    }

    const aliases: PathAlias[] = []
    for (const [alias, targetList] of Object.entries(paths)) {
      if (!Array.isArray(targetList)) continue
      const starIndex = alias.indexOf('*')
      aliases.push({
        prefix: starIndex === -1 ? alias : alias.slice(0, starIndex),
        suffix: starIndex === -1 ? '' : alias.slice(starIndex + 1),
        targets: targetList.filter(
          (target): target is string => typeof target === 'string',
        ),
      })
    }

    aliases.sort((a, b) => b.prefix.length - a.prefix.length)
    return { aliases, baseDir: resolve(configDir, baseUrl) }
  } catch {
    return { aliases: [], baseDir: configDir }
  }
}

function stripTrailingCommas(json: string): string {
  return json.replace(/,\s*([}\]])/g, '$1')
}

function resolveImport(
  specifier: string,
  importer: string,
  context: ResolveContext,
): string | undefined {
  const candidateBases: string[] = []

  if (specifier.startsWith('.')) {
    candidateBases.push(resolve(dirname(importer), specifier))
  } else if (specifier.startsWith('/')) {
    candidateBases.push(resolve(specifier))
  } else {
    candidateBases.push(...resolveAliasCandidates(specifier, context))
  }

  for (const candidateBase of candidateBases) {
    const resolvedFile = resolveFile(candidateBase)
    if (!resolvedFile) continue
    if (!context.files.has(resolvedFile)) continue
    if (!isInsideDir(context.srcDir, resolvedFile)) continue
    return resolvedFile
  }

  return undefined
}

function resolveAliasCandidates(
  specifier: string,
  context: ResolveContext,
): string[] {
  const candidates: string[] = []
  for (const alias of context.aliases) {
    if (
      !specifier.startsWith(alias.prefix) ||
      !specifier.endsWith(alias.suffix)
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
  }
  return candidates
}

function resolveFile(candidateBase: string): string | undefined {
  const ext = extname(candidateBase)
  if (ext) {
    if (!jsExtensions.includes(ext)) return undefined
    return isFile(candidateBase) ? resolve(candidateBase) : undefined
  }

  for (const jsExtension of jsExtensions) {
    const candidate = `${candidateBase}${jsExtension}`
    if (isFile(candidate)) return resolve(candidate)
  }
  for (const jsExtension of jsExtensions) {
    const candidate = join(candidateBase, `index${jsExtension}`)
    if (isFile(candidate)) return resolve(candidate)
  }

  return undefined
}

function isFile(path: string): boolean {
  try {
    return statSync(path).isFile()
  } catch {
    return false
  }
}

function isInsideDir(dir: string, file: string): boolean {
  const relPath = relative(dir, file)
  return relPath === '' || (!relPath.startsWith('..') && !isAbsolute(relPath))
}

function findClosedCycles(
  files: string[],
  roots: Set<string>,
  staticImporters: Map<string, Set<string>>,
): Set<string> {
  const parents = new Map<string, string>()
  for (const file of files) {
    if (roots.has(file)) continue
    const importers = staticImporters.get(file)
    if (importers?.size !== 1) continue
    const [importer] = importers
    parents.set(file, importer)
  }

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

if (import.meta.main) {
  const [srcDirArg, cwdArg = process.cwd(), tsconfigPathArg, outFileArg] =
    process.argv.slice(2)

  if (!srcDirArg) {
    console.error(
      'Usage: bun packages/next-plugin/src/import-graph.ts <srcDir> [cwd] [tsconfigPath] [outFile]',
    )
    process.exit(1)
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
