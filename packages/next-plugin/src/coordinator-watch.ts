import { type FSWatcher, type Stats, statSync, watch } from 'node:fs'
import { dirname, relative, resolve, sep } from 'node:path'
import { clearTimeout, setTimeout } from 'node:timers'

export interface SourceWatcher {
  close(): void
}

/**
 * Recursive source watches are backed by nonrecursive ancestor hints so a
 * missing or replaced source directory can acquire a fresh inode watch.
 * No watcher or debounce timer keeps the process alive.
 */
export function watchSources(options: {
  readonly roots: readonly string[]
  readonly debounceMs: number
  onChange(): void
  onError(error: Error): void
}): SourceWatcher {
  const roots = [...new Set(options.roots.map((root) => resolve(root)))]
  const watchers = new Map<
    string,
    {
      readonly path: string
      readonly watcher: FSWatcher
      readonly identity: string
    }
  >()
  let closed = false
  let timer: ReturnType<typeof setTimeout> | undefined
  const report = (error: unknown) => {
    if (!(error instanceof Error)) throw error
    if (!('code' in error && error.code === 'ENOENT')) options.onError(error)
  }
  const directory = (path: string): Stats | undefined => {
    try {
      const stat = statSync(path, { throwIfNoEntry: false })
      return stat?.isDirectory() ? stat : undefined
    } catch (error) {
      report(error)
      return undefined
    }
  }
  // Include the nearest existing ancestor's parent to observe its replacement
  // too, without recursively walking unrelated output or dependency trees.
  const boundaries = roots.map((root) => {
    let path = dirname(root)
    while (!directory(path) && dirname(path) !== path) path = dirname(path)
    return dirname(path)
  })
  const schedule = () => {
    if (closed) return
    clearTimeout(timer)
    timer = setTimeout(() => {
      refresh()
      options.onChange()
    }, options.debounceMs)
    timer.unref()
  }
  const attach = (path: string, recursive: boolean) => {
    const key = `${recursive}:${path}`
    const stat = directory(path)
    const identity = stat && `${stat.dev}:${stat.ino}:${stat.birthtimeMs}`
    const previous = watchers.get(key)
    if (previous?.identity === identity) return
    previous?.watcher.close()
    watchers.delete(key)
    if (!identity) return
    try {
      const watcher = watch(path, { recursive }, (_event, filename) => {
        if (closed || watchers.get(key)?.watcher !== watcher) return
        const changed =
          filename === null ? path : resolve(path, filename.toString())
        if (
          recursive ||
          changed === path ||
          roots.some((root) => {
            const child = relative(path, root).split(sep)[0]
            return resolve(path, child) === changed
          })
        ) {
          refresh()
          schedule()
        }
      })
      watcher.on('error', (error) => {
        if (closed || watchers.get(key)?.watcher !== watcher) return
        watcher.close()
        watchers.delete(key)
        report(error)
        schedule()
      })
      watcher.unref()
      watchers.set(key, { path, watcher, identity })
    } catch (error) {
      report(error)
    }
  }
  const refresh = () => {
    // Release the old subtree before attaching any replacement ancestor: some
    // watch backends share handles between overlapping directory watches.
    for (const [key, entry] of watchers) {
      const stat = directory(entry.path)
      if (
        stat &&
        entry.identity === `${stat.dev}:${stat.ino}:${stat.birthtimeMs}`
      )
        continue
      entry.watcher.close()
      watchers.delete(key)
    }
    roots.forEach((root, index) => {
      const ancestors: string[] = []
      let path = dirname(root)
      while (true) {
        ancestors.push(path)
        if (path === boundaries[index] || dirname(path) === path) break
        path = dirname(path)
      }
      for (const ancestor of ancestors.reverse()) attach(ancestor, false)
      attach(root, true)
    })
  }
  refresh()
  return {
    close() {
      closed = true
      clearTimeout(timer)
      for (const { watcher } of watchers.values()) watcher.close()
      watchers.clear()
    },
  }
}
