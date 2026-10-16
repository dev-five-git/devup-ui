import { watch } from 'node:fs'

/**
 * Watches the files of the project's Tailwind CSS, and calls `reload` when one
 * changes. `reload` gives the files to watch from then on; each file is
 * watched once.
 */
export function createTailwindWatcher(
  reload: () => Promise<string[]>,
  watchFile: (
    file: string,
    options: { persistent: false },
    listener: () => void,
  ) => {
    close(): void
    on(event: 'error', listener: () => void): unknown
  } = watch,
) {
  const watched = new Map<string, ReturnType<typeof watchFile>>()
  let active = true
  let pending = Promise.resolve()
  function watchFiles(files: string[]) {
    if (!active) return
    for (const file of files) {
      if (watched.has(file)) continue
      const watcher = watchFile(file, { persistent: false }, () => {
        if (!active) return
        pending = pending.then(async () => {
          if (active) watchFiles(await reload())
        })
      })
      watched.set(file, watcher)
      // A file that is removed or cannot be watched no longer rebuilds
      watcher.on('error', () => {
        watcher.close()
        watched.delete(file)
      })
    }
  }
  return Object.assign(watchFiles, {
    close() {
      active = false
      for (const watcher of watched.values()) watcher.close()
      watched.clear()
      return pending
    },
  })
}
