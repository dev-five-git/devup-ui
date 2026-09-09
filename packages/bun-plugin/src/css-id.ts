import { basename, dirname, resolve } from 'node:path'

/**
 * Namespace the extracted stylesheet is resolved into.
 *
 * Bun persists transpiled modules in a machine-wide on-disk cache
 * (`<bun cache>/@t@`) that is keyed by module contents, with plugin-resolved
 * import specifiers already baked in; the key covers neither the cwd nor the
 * importing file. Two checkouts of one repository — git worktrees, a CI
 * matrix, sibling clones — hold byte-identical sources, so they share cache
 * entries. Anything derived from `process.cwd()` that reaches an `onResolve`
 * result therefore leaks into the other checkout:
 *
 *     error: Cannot find module '<other checkout>/df/devup-ui/devup-ui.css'
 *       from '<this checkout>/src/Component.tsx'
 *
 * A namespaced id carries no filesystem path, so it is identical in every
 * checkout and sharing a cache entry is harmless.
 */
export const cssNamespace = 'devup-ui'

/** Directory, inside the plugin's dist dir, holding the extracted stylesheet. */
export const cssDirName = 'devup-ui'

const cssFileName = /^devup-ui(?:-\d+)?\.css$/

export interface DevupCssId {
  path: string
  namespace: string
}

/**
 * Maps a `<distDir>/<cssDirName>/devup-ui[-N].css` import — the one the
 * extractor injects into every transformed source file — onto a virtual module
 * id, and returns `undefined` for any other stylesheet so it keeps Bun's normal
 * resolution.
 *
 * The decision is made on the *shape* of the resolved directory rather than on
 * equality with this checkout's absolute css dir. That keeps the function
 * independent of `process.cwd()`, and it also repairs specifiers that an older
 * plugin version already baked into a shared cache entry as another checkout's
 * absolute path.
 */
export function resolveCssId(
  specifier: string,
  importer: string | undefined,
  distDir: string,
): DevupCssId | undefined {
  const fileName = basename(specifier).split('?')[0]
  if (!fileName || !cssFileName.test(fileName)) return undefined

  const resolvedDir = dirname(
    importer ? resolve(dirname(importer), specifier) : resolve(specifier),
  )
  if (
    basename(resolvedDir) !== cssDirName ||
    basename(dirname(resolvedDir)) !== distDir
  )
    return undefined

  return { path: fileName, namespace: cssNamespace }
}
