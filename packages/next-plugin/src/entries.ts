import { existsSync } from 'node:fs'
import { basename, dirname, join, resolve, sep } from 'node:path'

const DEFAULT_PAGE_EXTENSIONS = ['jsx', 'js', 'tsx', 'ts'] as const
const APP_SHELLS = new Set([
  'layout',
  'template',
  'default',
  'loading',
  'error',
  'not-found',
  'global-error',
  'forbidden',
  'unauthorized',
])
const RUNTIME_CONVENTIONS = new Set([
  'middleware',
  'proxy',
  'instrumentation',
  'instrumentation-client',
])

/**
 * Select explicit Next entry modules from the graph's absolute file list.
 * Router presence, not graph contents, decides root versus src precedence.
 * Import traversal belongs to computeReachableFiles, not this selector.
 */
export function collectNextEntries({
  root,
  files,
  pageExtensions = DEFAULT_PAGE_EXTENSIONS,
}: {
  readonly root: string
  readonly files: readonly string[]
  readonly pageExtensions?: readonly string[]
}): string[] {
  const projectRoot = resolve(root)
  const [appDir, pagesDir] = ['app', 'pages'].map((router) =>
    [join(projectRoot, router), join(projectRoot, 'src', router)].find((dir) =>
      existsSync(dir),
    ),
  )
  const routerDir = pagesDir ?? appDir
  if (!routerDir) return []
  const runtimeDir = dirname(routerDir)
  const extensions = [...pageExtensions].sort((a, b) => b.length - a.length)

  const entries = new Set<string>()
  const appModules: { readonly file: string; readonly name: string }[] = []
  const renderDirs = new Set<string>()
  for (const input of files) {
    const file = resolve(projectRoot, input)
    const filename = basename(file)
    const extension = extensions.find((ext) => filename.endsWith(`.${ext}`))
    if (extension === undefined) continue
    const name = filename.slice(0, -(extension.length + 1))
    if (dirname(file) === runtimeDir && RUNTIME_CONVENTIONS.has(name)) {
      entries.add(file)
    }
    if (pagesDir && file.startsWith(`${pagesDir}${sep}`)) entries.add(file)
    if (!appDir || !file.startsWith(`${appDir}${sep}`)) continue
    const segments = file.slice(appDir.length + 1).split(sep)
    if (segments.slice(0, -1).some((segment) => segment.startsWith('_')))
      continue
    appModules.push({ file, name })
    const rootModule = dirname(file) === appDir
    const metadata =
      (rootModule && (name === 'robots' || name === 'manifest')) ||
      /^(?:sitemap|(?:icon|apple-icon|opengraph-image|twitter-image)\d?)$/.test(
        name,
      )
    const renderEntry =
      name === 'page' ||
      (rootModule && (name === 'not-found' || name === 'global-not-found'))
    if (name === 'route' || metadata || renderEntry) entries.add(file)
    if (!renderEntry) continue
    for (let dir = dirname(file); ; dir = dirname(dir)) {
      renderDirs.add(dir)
      if (dir === appDir) break
    }
  }

  // A slot with only default UI is still rendered by its active parent layout.
  // Dead sibling segments are not render ancestors and never become entries.
  appModules.sort((a, b) => a.file.length - b.file.length)
  for (const { file, name } of appModules) {
    const dir = dirname(file)
    if (
      name === 'default' &&
      basename(dir).startsWith('@') &&
      renderDirs.has(dirname(dir))
    ) {
      renderDirs.add(dir)
    }
  }
  for (const { file, name } of appModules) {
    if (APP_SHELLS.has(name) && renderDirs.has(dirname(file))) entries.add(file)
  }
  return [...entries].sort()
}
