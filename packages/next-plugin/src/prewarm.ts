import { existsSync, readFileSync } from 'node:fs'
import { extname, isAbsolute, relative, resolve } from 'node:path'

import {
  createModuleResolver,
  parseImports,
  type StaticImportGraph,
} from '@devup-ui/plugin-utils'

const EXTRACTABLE_EXTENSION = /\.(?:tsx?|jsx?|mjs)$/

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
  include: string[],
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

function toKey(cwd: string, filename: string): string {
  return relative(cwd, resolve(cwd, filename)).replaceAll('\\', '/')
}

export interface CollectProductionPrewarmFilesOptions {
  cwd: string
  graph: StaticImportGraph
  expectedBaseFiles: string[]
  libPackage: string
  include: string[]
}

/**
 * Build the complete deterministic production extraction set before a bundler
 * requests its first CSS module.
 *
 * `computeCompiledFiles` is intentionally route-aware, but a bundler can also
 * compile files hidden behind template imports / MDX and package entries that
 * live outside `srcDir`. Prewarming every extractable source file plus the
 * external package closures accepted by the loader makes the first snapshot
 * independent of Turbopack scheduling. Per-file mode still only imports chunks
 * that the bundler reaches; single-CSS mode intentionally contains the whole
 * application stylesheet.
 */
export function collectProductionPrewarmFiles({
  cwd,
  graph,
  expectedBaseFiles,
  libPackage,
  include,
}: CollectProductionPrewarmFilesOptions): string[] {
  const resolvedCwd = resolve(cwd)
  const files = new Set(
    expectedBaseFiles.map((filename) => toKey(resolvedCwd, filename)),
  )

  for (const filename of graph.files) {
    files.add(toKey(resolvedCwd, filename))
  }

  const resolveModule = createModuleResolver({ cwd: resolvedCwd })
  const visited = new Set<string>()
  const queue = [...(graph.externalImports ?? [])].flatMap(
    ([importer, specifiers]) =>
      [...specifiers].sort().map((specifier) => ({ importer, specifier })),
  )
  for (const { importer, specifier } of queue) {
    const local = specifier.startsWith('.') || isAbsolute(specifier)
    if (!local) {
      const packageName = packageNameFromSpecifier(specifier)
      if (!packageName || !isPrewarmPackage(packageName, libPackage, include))
        continue
    }
    const module = resolveModule(specifier, importer)
    if (!module || module.ignored === true) continue
    const filename = preferEsmFile(module.path)
    if (!EXTRACTABLE_EXTENSION.test(filename) || visited.has(filename)) continue
    visited.add(filename)
    files.add(toKey(resolvedCwd, filename))
    const source =
      filename === module.path ? module.code : readFileSync(filename, 'utf-8')
    for (const imported of parseImports(filename, source)) {
      queue.push({ importer: filename, specifier: imported.specifier })
    }
  }

  return [...files].sort()
}
