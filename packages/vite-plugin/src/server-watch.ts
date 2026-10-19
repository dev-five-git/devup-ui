import type { createViteReservation } from './reservation'

interface Watcher {
  add(paths: string[]): unknown
  on(event: 'all', listener: (event: string, file: string) => void): unknown
  off(event: 'all', listener: (event: string, file: string) => void): unknown
}

export function createServerWatch(
  watcher: Watcher,
  reservation: Pick<
    ReturnType<typeof createViteReservation>,
    'watch' | 'invalidate'
  >,
  unlink: (file: string) => void,
) {
  const registered = new Set<string>()
  const add = (paths: readonly string[]) => {
    const added = [...new Set(paths)].filter((path) => !registered.has(path))
    if (!added.length) return
    watcher.add(added)
    for (const path of added) registered.add(path)
  }
  const changed = (event: string, file: string) => {
    reservation.invalidate(file)
    if (event === 'unlink') unlink(file)
  }
  reservation.watch(add)
  watcher.on('all', changed)
  return {
    add,
    close() {
      watcher.off('all', changed)
      reservation.watch(() => {})
    },
  }
}
