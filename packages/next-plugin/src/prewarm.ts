import { existsSync, readFileSync } from 'node:fs'
import { extname, relative, resolve } from 'node:path'

import {
  createModuleResolver,
  type ModuleAliases,
  type PrepareSource,
  type ResolvedModule,
  type StaticImportGraph,
} from '@devup-ui/plugin-utils'

import { locatedError } from './build-error'

/** Every extension the Turbopack source rule and the prewarm accept. */
export const EXTRACTABLE_EXTENSION = /\.[cm]?[jt]sx?$/
const DECLARATION_FILE = /\.d\.[cm]?ts$/
const IMPORT_SPECIFIER =
  /(?:\bfrom\s*|\bimport\s*\(?\s*|\brequire\s*\(\s*)(['"])([^'"\n]+)\1/g

function packageNameFromSpecifier(specifier: string): string | undefined {
  if (specifier.startsWith('#') || specifier.startsWith('node:')) {
    return undefined
  }

  const [first, second] = specifier.split('/')
  if (!first) return undefined
  if (!first.startsWith('@')) return first
  return second ? `${first}/${second}` : undefined
}

function isPrewarmPackage(
  packageName: string,
  libPackage: string,
  include: readonly string[],
): boolean {
  const configuredPackage = packageNameFromSpecifier(libPackage)
  return (
    packageName.startsWith('@devup-ui/') ||
    packageName.startsWith('@devup-editor/') ||
    packageName === configuredPackage ||
    include.some(
      (included) => packageName === packageNameFromSpecifier(included),
    )
  )
}

function preferEsmFile(filename: string): string {
  if (extname(filename) !== '.cjs') return filename
  const esmFilename = `${filename.slice(0, -4)}.mjs`
  return existsSync(esmFilename) ? esmFilename : filename
}

function toKey(root: string, filename: string): string {
  return relative(root, resolve(root, filename)).replaceAll('\\', '/')
}

function isExtractable(filename: string): boolean {
  return (
    EXTRACTABLE_EXTENSION.test(filename) && !DECLARATION_FILE.test(filename)
  )
}

interface PackageWalk {
  readonly root: string
  readonly libPackage: string
  readonly include: readonly string[]
  readonly resolveModule: (
    specifier: string,
    importer: string,
  ) => ResolvedModule | undefined
  readonly files: Set<string>
  readonly seen: Set<string>
}

function resolveLocated(
  walk: PackageWalk,
  specifier: string,
  importer: string,
): ResolvedModule | undefined {
  try {
    return walk.resolveModule(specifier, importer)
  } catch (cause) {
    throw locatedError({
      file: importer,
      what: 'devup-ui prewarm',
      code: specifier,
      needs: 'a resolvable package with a readable manifest and entry',
      cause,
    })
  }
}

function followSpecifier(
  walk: PackageWalk,
  specifier: string,
  importer: string,
): void {
  if (!specifier.startsWith('.')) {
    const name = packageNameFromSpecifier(specifier)
    if (!name || !isPrewarmPackage(name, walk.libPackage, walk.include)) return
  }
  const resolved = resolveLocated(walk, specifier, importer)
  if (resolved) addPackageFile(walk, resolved)
}

/** Add a package file and, through the resolver, what it imports in turn. */
function addPackageFile(walk: PackageWalk, entry: ResolvedModule): void {
  const filename = preferEsmFile(entry.path)
  if (!isExtractable(filename) || walk.seen.has(filename)) return
  walk.seen.add(filename)
  walk.files.add(toKey(walk.root, filename))
  const source =
    filename === entry.path
      ? entry.code
      : (walk.resolveModule(filename, filename)?.code ??
        readFileSync(filename, 'utf-8'))
  for (const [, , specifier] of source.matchAll(IMPORT_SPECIFIER)) {
    followSpecifier(walk, specifier, filename)
  }
}

export interface CollectPrewarmFilesOptions {
  root: string
  graph: StaticImportGraph
  expectedBaseFiles: readonly string[]
  libPackage: string
  include: readonly string[]
  /** Also prewarm every source file of the graph, reachable or not */
  prewarmAll: boolean
  readonly resolver?: {
    readonly prepareSource?: PrepareSource
    readonly alias?: ModuleAliases
    readonly conditions?: readonly string[]
    readonly includeMdx?: readonly string[]
  }
}

/**
 * The deterministic extraction set that runs before the bundler can request
 * its first stylesheet: the proven compiled closure, plus the packages that
 * closure imports and that the loader would extract (`@devup-ui/*`, the
 * configured package, `include`), followed through their own imports.
 *
 * A source file nothing compiled imports is not extracted: it adds no CSS and
 * cannot fail the build. `prewarmAll` opts into the whole tree instead, for
 * files the graph cannot connect (template imports, MDX).
 */
export function collectPrewarmFiles({
  root,
  graph,
  expectedBaseFiles,
  libPackage,
  include,
  prewarmAll,
  resolver,
}: CollectPrewarmFilesOptions): string[] {
  const resolvedRoot = resolve(root)
  const files = new Set(
    (prewarmAll
      ? [...expectedBaseFiles, ...graph.files]
      : expectedBaseFiles
    ).map((filename) => toKey(resolvedRoot, filename)),
  )
  const reached = new Set([...files].map((key) => resolve(resolvedRoot, key)))
  const walk: PackageWalk = {
    root: resolvedRoot,
    libPackage,
    include,
    resolveModule: createModuleResolver({ cwd: resolvedRoot, ...resolver }),
    files,
    seen: new Set(),
  }

  for (const [importer, specifiers] of graph.externalImports ?? []) {
    if (!reached.has(importer)) continue
    for (const specifier of [...specifiers].sort()) {
      followSpecifier(walk, specifier, importer)
    }
  }

  return [...files].sort()
}
