import { dirname, relative } from 'node:path'

import type { StaticImportGraph } from './import-graph'

const SOURCE = '(?:tsx|ts|mts|cts|jsx|js|mjs|cjs)'
/** An App Router file Next renders around every route below its directory */
const APP_SHELL = new RegExp(
  `(?:^|/)app/(?:.*/)?(?:layout|template)\\.${SOURCE}$`,
)
/** The App Router files Next renders inside the layouts above them */
const APP_WRAPPED = new RegExp(
  `(?:^|/)app/(?:.*/)?(?:page|default|not-found|forbidden|unauthorized|loading|error)\\.${SOURCE}$`,
)
/** The Pages Router file Next renders around every page */
const PAGES_SHELL = new RegExp(`(?:^|/)pages/_app\\.${SOURCE}$`)
/** The Pages Router pages Next renders inside `_app` */
const PAGES_WRAPPED = new RegExp(
  `(?:^|/)pages/(?!api/|_document\\.).*\\.${SOURCE}$`,
)

interface Shell {
  readonly file: string
  /** POSIX directory, relative to the root, holding the routes it wraps */
  readonly dir: string
  /** The routes of its kind it wraps */
  readonly wraps: RegExp
}

function toPosix(root: string, file: string): string {
  return relative(root, file).replaceAll('\\', '/')
}

/** `child` is `dir` itself or inside it (both POSIX, root-relative) */
function isWithin(dir: string, child: string): boolean {
  return child === dir || child.startsWith(`${dir}/`)
}

function shellOf(root: string, file: string): Shell | undefined {
  const path = toPosix(root, file)
  if (APP_SHELL.test(path))
    return { file, dir: dirname(path), wraps: APP_WRAPPED }
  if (PAGES_SHELL.test(path))
    return { file, dir: dirname(path), wraps: PAGES_WRAPPED }
  return undefined
}

/**
 * The routes of a Next.js build as atom hoisting should count them. Next
 * compiles each `layout`, `template` and `pages/_app` as an entry of its own,
 * yet renders it with every route below its directory: as a route, the files
 * only it imports count as reached by one route and keep a stylesheet of their
 * own, which every page loads anyway.
 *
 * A shell that wraps every route of its router (the root layout, `_app`) is
 * no route here; each of those routes reaches it instead, so what it imports
 * reaches every route and is shared. A shell that wraps only some routes (a
 * nested layout) stays a route of its own, so its styles never load on the
 * pages outside it. Other routes (lazy-loaded files, other entries) are kept
 * as they are and reach no shell, so what only they import stays per-file.
 *
 * Returns a graph that differs only in those added route-to-shell edges.
 */
export function withRouteShells(
  graph: StaticImportGraph,
  routes: readonly string[],
  root: string,
): { graph: StaticImportGraph; routes: string[] } {
  const shells: Shell[] = []
  const pages: string[] = []
  for (const route of routes) {
    const shell = shellOf(root, route)
    if (shell) shells.push(shell)
    else pages.push(route)
  }
  const wrapped = new Map<string, string[]>()
  const kept = [...pages]
  for (const shell of shells) {
    const ofKind = pages.filter((page) => shell.wraps.test(toPosix(root, page)))
    const below = ofKind.filter((page) =>
      isWithin(shell.dir, dirname(toPosix(root, page))),
    )
    if (below.length === 0 || below.length < ofKind.length) {
      kept.push(shell.file)
      continue
    }
    for (const page of below)
      wrapped.set(page, [...(wrapped.get(page) ?? []), shell.file])
  }
  if (wrapped.size === 0) return { graph, routes: kept }
  const staticImports = new Map(graph.staticImports)
  for (const [page, files] of wrapped)
    staticImports.set(
      page,
      new Set([...(graph.staticImports.get(page) ?? []), ...files]),
    )
  return { graph: { ...graph, staticImports }, routes: kept }
}
