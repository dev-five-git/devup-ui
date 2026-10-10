import { lstatSync, readdirSync, realpathSync, statSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'

import { createDirectoryExclusion } from './directory-exclusion'
import { compareCodePoints } from './import-graph'
import type { SourceSelectionOptions } from './source-selection'
import { isSelectedSource } from './source-selection'

export interface ProductionSourceFilesOptions extends SourceSelectionOptions {
  readonly roots: readonly string[]
  readonly sourceFiles?: readonly string[]
  readonly entries?: readonly string[]
  readonly exclude?: readonly string[]
  readonly cwd?: string
}

export interface ProductionSourceFile {
  readonly path: string
  readonly realPath: string
}

export function enumerateProductionSourceFiles({
  roots,
  sourceFiles = [],
  entries = [],
  exclude,
  cwd = process.cwd(),
  includeMdx,
}: ProductionSourceFilesOptions): readonly ProductionSourceFile[] {
  const excluded = createDirectoryExclusion(exclude)
  const files = new Map<string, ProductionSourceFile>()

  function visit(
    path: string,
    ancestry: ReadonlySet<string>,
    optional = false,
  ): void {
    if (excluded(path)) return
    const lexical = lstatSync(path, { throwIfNoEntry: !optional })
    if (!lexical) return
    const realPath = realpathSync.native(path)
    if (excluded(realPath)) return
    const physical = lexical.isSymbolicLink() ? statSync(realPath) : lexical
    if (physical.isDirectory()) {
      if (ancestry.has(realPath)) return
      // Only active ancestry closes cycles; sibling aliases retain their IDs.
      const next = new Set([...ancestry, realPath])
      for (const entry of readdirSync(path, { withFileTypes: true })) {
        if (entry.name === 'node_modules') continue
        visit(join(path, entry.name), next)
      }
    } else if (physical.isFile() && isSelectedSource(path, includeMdx)) {
      files.set(path, { path, realPath })
    }
  }

  for (const root of roots) visit(resolve(cwd, root), new Set(), true)
  for (const source of sourceFiles) visit(resolve(cwd, source), new Set())
  for (const entry of entries) {
    const path = resolve(cwd, entry)
    visit(path, new Set())
    visit(dirname(path), new Set())
  }
  return [...files.values()].sort((a, b) => compareCodePoints(a.path, b.path))
}
