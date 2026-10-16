import { watch } from 'node:fs'

/**
 * Watches the files of the project's Tailwind CSS, and calls `reload` when one
 * changes. `reload` gives the files to watch from then on; each file is
 * watched once.
 */
export function createTailwindWatcher(
  reload: () => Promise<string[]>,
  watchFile: typeof watch = watch,
) {
  const watched = new Set<string>()
  function watchFiles(files: string[]) {
    for (const file of files) {
      if (watched.has(file)) continue
      watched.add(file)
      const watcher = watchFile(file, { persistent: false }, () => {
        void reload().then(watchFiles)
      })
      // A file that is removed or cannot be watched no longer rebuilds
      watcher.on('error', () => {
        watcher.close()
        watched.delete(file)
      })
    }
  }
  return watchFiles
}
