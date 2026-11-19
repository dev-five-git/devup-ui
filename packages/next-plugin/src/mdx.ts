import { type Dirent, existsSync, readdirSync, readFileSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'

import { parseImports } from '@devup-ui/plugin-utils'

const mdxFileRegex = /\.mdx$/i
// The App Router segment files and Pages Router modules Next compiles as
// routes, as `computeCompiledFiles` takes them, in their MDX form.
const appRouterMdxRegex =
  /(^|\/)(page|layout|template|default|loading|error|not-found|forbidden|unauthorized|global-error|global-not-found)\.mdx$/i
const pagesRouterMdxRegex = /^pages\/.+\.mdx$/i
const fenceRegex = /^ {0,3}(`{3,}|~{3,})/
const esmStartRegex = /^(?:import|export)\b/

function toPosixRelative(from: string, to: string): string {
  return relative(from, to).replaceAll('\\', '/')
}

/**
 * The `.mdx` files under `root`, as absolute paths, skipping `node_modules`
 * and dot directories like the source scan does.
 */
export function listMdxFiles(root: string): string[] {
  const files: string[] = []
  const visit = (dir: string): void => {
    let entries: Dirent[]
    try {
      entries = readdirSync(dir, { withFileTypes: true })
    } catch {
      return
    }
    for (const entry of entries) {
      const path = join(dir, entry.name)
      if (entry.isDirectory()) {
        if (entry.name !== 'node_modules' && !entry.name.startsWith('.'))
          visit(path)
      } else if (entry.isFile() && mdxFileRegex.test(entry.name)) {
        files.push(resolve(path))
      }
    }
  }
  visit(root)
  return files.sort()
}

/**
 * The ESM of an MDX document: the blocks that start with `import` or `export`
 * at the start of a line after a blank one, up to the next blank line, outside
 * fenced code. MDX takes the same blocks as its module code, so their imports
 * are the document's.
 */
export function mdxEsm(source: string): string {
  const esm: string[] = []
  let fence: string | undefined
  let inEsm = false
  let afterBlank = true
  for (const line of source.split(/\r?\n/)) {
    if (fence !== undefined) {
      if (line.trimStart().startsWith(fence)) fence = undefined
      afterBlank = false
      continue
    }
    const blank = line.trim() === ''
    if (inEsm && !blank) {
      esm.push(line)
      continue
    }
    inEsm = false
    const open = fenceRegex.exec(line)
    if (open) fence = open[1]
    else if (afterBlank && esmStartRegex.test(line)) {
      inEsm = true
      esm.push(line)
    }
    afterBlank = blank
  }
  return esm.join('\n')
}

/** The existing `.mdx` files `importer` imports by a relative specifier. */
function importedMdx(importer: string, source: string): string[] {
  return parseImports(importer, source)
    .map(({ specifier }) => specifier)
    .filter(
      (specifier) => specifier.startsWith('.') && mdxFileRegex.test(specifier),
    )
    .map((specifier) => resolve(dirname(importer), specifier))
    .filter((path) => existsSync(path))
}

export interface CollectCompiledMdxFilesOptions {
  cwd: string
  /** The directory Next reads routes from (`app`/`pages` live in it). */
  srcDir: string
  /** Next's `pageExtensions`; `.mdx` route files are compiled only with `mdx`. */
  pageExtensions: readonly string[] | undefined
  /** The compiled source files (cwd-relative POSIX), as `computeCompiledFiles` returns them. */
  compiledFiles: readonly string[]
}

/**
 * The `.mdx` files a Turbopack build compiles: MDX route files when
 * `pageExtensions` lists `mdx`, the MDX documents compiled sources import,
 * and the documents those import in turn. Turbopack hands their compiled
 * JavaScript to the source rule, so each one reaches the coordinator under
 * its own path (cwd-relative POSIX, as returned). An import through a path
 * alias is not followed.
 */
export function collectCompiledMdxFiles({
  cwd,
  srcDir,
  pageExtensions,
  compiledFiles,
}: CollectCompiledMdxFilesOptions): string[] {
  const queue: string[] = []
  if (pageExtensions?.includes('mdx')) {
    for (const file of listMdxFiles(srcDir)) {
      const route = toPosixRelative(srcDir, file)
      if (appRouterMdxRegex.test(route) || pagesRouterMdxRegex.test(route))
        queue.push(file)
    }
  }
  for (const filename of compiledFiles) {
    const file = resolve(cwd, filename)
    const source = readFileSync(file, 'utf-8')
    if (!source.includes('.mdx')) continue
    queue.push(...importedMdx(file, source))
  }
  const compiled = new Set<string>()
  for (const file of queue) {
    if (compiled.has(file)) continue
    compiled.add(file)
    const esm = mdxEsm(readFileSync(file, 'utf-8'))
    queue.push(...importedMdx(`${file}.mjs`, esm))
  }
  return [...compiled].map((file) => toPosixRelative(cwd, file)).sort()
}

/**
 * The `.mdx` files under `roots` whose ESM mentions one of `needles`,
 * cwd-relative POSIX: the documents that extract classes, numbered with the
 * sources. A needle only in prose or a code block does not count.
 */
export function collectNumberedMdxFiles(
  roots: readonly string[],
  cwd: string,
  needles: readonly string[],
): string[] {
  return roots
    .flatMap(listMdxFiles)
    .filter((file) => {
      const esm = mdxEsm(readFileSync(file, 'utf-8'))
      return needles.some((needle) => esm.includes(needle))
    })
    .map((file) => toPosixRelative(cwd, file))
}
