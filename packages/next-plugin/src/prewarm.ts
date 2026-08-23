import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { extname, join, relative, resolve } from 'node:path'

import type { StaticImportGraph } from '@devup-ui/plugin-utils'

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
 * Build the deterministic production extraction set used before Turbopack can
 * request its first CSS module.
 *
 * `computeCompiledFiles` is intentionally route-aware, but a bundler can also
 * compile files hidden behind template imports / MDX and package entries that
 * live outside `srcDir`. Prewarming every extractable source file plus the
 * external package entries accepted by the loader makes the first snapshot
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

  const externalSpecifiers = new Set<string>()
  for (const specifiers of graph.externalImports?.values() ?? []) {
    for (const specifier of specifiers) externalSpecifiers.add(specifier)
  }

  const requireFromProject = createRequire(join(resolvedCwd, 'package.json'))
  for (const specifier of [...externalSpecifiers].sort()) {
    const packageName = packageNameFromSpecifier(specifier)
    if (!packageName || !isPrewarmPackage(packageName, libPackage, include)) {
      continue
    }

    try {
      const filename = preferEsmFile(requireFromProject.resolve(specifier))
      if (!EXTRACTABLE_EXTENSION.test(filename)) continue
      files.add(toKey(resolvedCwd, filename))
    } catch {
      // Resolution is best-effort, matching the static graph pre-pass. The
      // loader remains the fallback for packages resolved only by Turbopack.
    }
  }

  return [...files].sort()
}
