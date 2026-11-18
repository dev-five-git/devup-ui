import { existsSync, type FSWatcher, watch } from 'node:fs'
import { basename, dirname, resolve } from 'node:path'

/** The files a theme was loaded from */
export interface ThemeInputs {
  /**
   * devup.json and every file of its `extends` chain, existing or not: a
   * change, a creation or a removal reloads the theme
   */
  readonly config: readonly string[]
  /**
   * The project's Tailwind CSS: a change reloads the theme, a removal does
   * not, as in the Vite plugin
   */
  readonly tailwind: readonly string[]
}

/**
 * Watches the files the theme is loaded from, and calls `reload` when one is
 * created, changed or removed. `reload` gives the files to watch from then
 * on; a file stays watched once watched. Each file is watched through its
 * directory, since a missing file cannot be watched and an editor may save
 * by replacing it. Changes seen while a reload runs bring one more reload
 * after it, so reloads never overlap.
 */
export function createConfigWatcher(
  reload: () => Promise<ThemeInputs>,
  watchDirectory: typeof watch = watch,
  exists: (path: string) => boolean = existsSync,
) {
  // Each file, and whether its removal reloads
  const files = new Map<string, boolean>()
  const directories = new Map<string, FSWatcher>()
  let reloading = false
  let changedAgain = false

  async function reloadUntilSettled() {
    reloading = true
    try {
      do {
        changedAgain = false
        try {
          watchFiles(await reload())
        } catch (error) {
          console.error(error)
        }
      } while (changedAgain)
    } finally {
      reloading = false
    }
  }

  function changed(directory: string, filename: string | null) {
    const affected = [...files].some(
      ([file, removalReloads]) =>
        dirname(file) === directory &&
        (filename === null || basename(file) === filename) &&
        (removalReloads || exists(file)),
    )
    if (!affected) return
    if (reloading) changedAgain = true
    else void reloadUntilSettled()
  }

  function watchDirectoryOf(file: string) {
    const directory = dirname(file)
    if (directories.has(directory)) return
    let watcher: FSWatcher
    try {
      watcher = watchDirectory(
        directory,
        { persistent: false, encoding: 'utf8' },
        (_event, filename) => changed(directory, filename),
      )
    } catch {
      // A directory that does not exist is watched once a reload names it
      // again after it is created
      return
    }
    directories.set(directory, watcher)
    // A directory that is removed or cannot be watched no longer reloads
    watcher.on('error', () => {
      watcher.close()
      directories.delete(directory)
    })
  }

  function watchFiles({ config, tailwind }: ThemeInputs) {
    for (const file of tailwind.map((path) => resolve(path)))
      if (!files.has(file)) files.set(file, false)
    for (const file of config.map((path) => resolve(path)))
      files.set(file, true)
    for (const file of files.keys()) watchDirectoryOf(file)
  }
  return watchFiles
}
