import { existsSync, readFileSync, realpathSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { listSourceFiles } from './import-graph'
import { MDX_FILE_RE } from './shared'

export interface CollectNumberedFilesOptions {
  /** Directories holding the project's own source files */
  roots: string[]
  /** Packages whose source the build extracts too (the `include` option) */
  include?: string[]
  /** Number MDX only when the caller extracts compiled MDX. Defaults to false. */
  includeMdx?: boolean
  cwd?: string
  /**
   * Only files whose text contains one of these are numbered, so files the
   * build never extracts do not take numbers (and lengthen class prefixes)
   */
  needles?: string[]
  /** The name the plugin extracts a file under, given its absolute path */
  toId?: (path: string) => string
}

/** Whether the text of the file contains one of the needles */
function usesAny(file: string, needles: string[]): boolean {
  const text = readFileSync(file, 'utf-8')
  return needles.some((needle) => text.includes(needle))
}

/** Where an included package lives, found the way the bundler finds it. */
function findPackageDir(cwd: string, name: string): string | undefined {
  let dir = resolve(cwd)
  for (;;) {
    const candidate = join(dir, 'node_modules', name)
    if (existsSync(candidate)) return realpathSync(candidate)
    const parent = resolve(dir, '..')
    if (parent === dir) return undefined
    dir = parent
  }
}

/**
 * Every file the build can extract, as the names the plugin extracts them
 * under: the project's source, then the source of each included package. The
 * list is the same on every run for the same files, and the engine numbers it
 * in path order, so file numbers (and the class prefixes they make) do not
 * depend on which file a bundler worker reaches first.
 */
export function collectNumberedFiles({
  roots,
  include = [],
  includeMdx = false,
  cwd = process.cwd(),
  needles,
  toId = (path) => path.replaceAll('\\', '/'),
}: CollectNumberedFilesOptions): string[] {
  const directories = [
    ...roots,
    ...include
      .map((name) => findPackageDir(cwd, name))
      .filter((dir): dir is string => dir !== undefined),
  ]
  return [
    ...new Set(
      directories.flatMap((dir) =>
        listSourceFiles(dir)
          .filter((file) => includeMdx || !MDX_FILE_RE.test(file))
          .filter((file) => !needles || usesAny(file, needles))
          .map(toId),
      ),
    ),
  ].sort()
}

export interface FileNumbering {
  seedFileMap(files: string[]): void
}

/**
 * Number `files` in path order. Files numbered before keep their numbers, so
 * running it again as files appear (in development) numbers only the new ones,
 * after the existing ones.
 */
export function seedFileNumbers(
  engine: FileNumbering,
  files: Parameters<FileNumbering['seedFileMap']>[0],
): void {
  if (files.length > 0) engine.seedFileMap(files)
}

/** What a file mentions when the build extracts it: the package, StyleX, and the packages Devup UI takes the place of */
export function extractedNeedles(
  libPackage: string,
  importAliases: object,
): string[] {
  return [libPackage, '@stylexjs/stylex', ...Object.keys(importAliases)]
}
