import { type Dirent, readdirSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'

import type { StaticImportGraph } from '@devup-ui/plugin-utils'

/** App Router files Next turns into a route of their own, by any extension */
const APP_ROUTE_FILE =
  /(^|\/)(page|route|default|not-found|forbidden|unauthorized|global-error|global-not-found)\.[^/]+$/
/** App Router files Next compiles around a route: they serve every page below */
const APP_SHELL_FILE =
  /(^|\/)(layout|template|loading|error)\.(tsx|ts|jsx|js|mts|cts|mjs|cjs)$/
/** Pages Router modules that are no route, but wrap every page */
const PAGES_NON_ROUTE = /^pages\/_(app|document)\.[^/]+$/

/** Every file under `directory`, as POSIX paths relative to `root`; none when it is missing */
function listFiles(root: string, directory: string): string[] {
  let entries: Dirent[]
  try {
    entries = readdirSync(directory, { withFileTypes: true })
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return []
    throw error
  }
  return entries.flatMap((entry) => {
    if (entry.name === 'node_modules' || entry.name.startsWith('.')) return []
    const path = join(directory, entry.name)
    if (entry.isDirectory()) return listFiles(root, path)
    return [relative(root, path).replaceAll('\\', '/')]
  })
}

/**
 * Whether the Next app under `srcDir` serves exactly one route, with every
 * source file loaded for it: one page (or route handler, or Pages Router
 * page) and the layouts around it, no `import()` that splits off a chunk of
 * its own, and no source file that route leaves out.
 *
 * Every stylesheet of such an app is loaded together on its one page, so it
 * gains nothing from a stylesheet per file. Anything the import graph cannot
 * see for sure (a route file it does not parse, such as `page.mdx`, a lazily
 * imported module, a file no route imports, a directory it cannot read)
 * answers `false`.
 */
export function isSingleRouteApp(
  srcDir: string,
  graph: StaticImportGraph,
): boolean {
  const root = resolve(srcDir)
  let appFiles: string[]
  let pagesFiles: string[]
  try {
    appFiles = listFiles(root, join(root, 'app'))
    pagesFiles = listFiles(root, join(root, 'pages'))
  } catch {
    return false
  }
  const routes = [
    ...appFiles.filter((file) => APP_ROUTE_FILE.test(file)),
    ...pagesFiles.filter((file) => !PAGES_NON_ROUTE.test(file)),
  ]
  if (routes.length !== 1 || graph.dynamicTargets.size > 0) return false
  const entries = [
    ...routes,
    ...appFiles.filter((file) => APP_SHELL_FILE.test(file)),
    ...pagesFiles.filter((file) => PAGES_NON_ROUTE.test(file)),
  ].map((file) => resolve(root, file))
  const reached = new Set<string>()
  for (let index = 0; index < entries.length; index += 1) {
    const file = entries[index]
    if (reached.has(file)) continue
    reached.add(file)
    entries.push(...(graph.staticImports.get(file) ?? []))
  }
  return graph.files.every((file) => reached.has(file))
}
