import { dirname } from 'node:path'

import type { ViteDevServer } from 'vite'

export function createMissingInputWatch() {
  const missingByEnvironment = new Map<object, Map<string, Set<string>>>()
  let server: ViteDevServer | undefined
  const changed = (file: string) => {
    const path = file.replaceAll('\\', '/')
    let affected = false
    for (const environment of Object.values(server?.environments ?? {})) {
      for (const [importer, inputs] of missingByEnvironment.get(environment) ??
        []) {
        if (!inputs.has(path)) continue
        for (const module of environment.moduleGraph.getModulesByFile(
          importer,
        ) ?? []) {
          environment.moduleGraph.invalidateModule(module)
          affected = true
        }
      }
    }
    if (affected) server?.ws.send({ type: 'full-reload' })
  }
  return {
    attach(current: ViteDevServer) {
      server = current
      current.watcher
        .on('add', changed)
        .on('change', changed)
        .on('unlink', changed)
    },
    start(importer: string, environment: object) {
      const importers =
        missingByEnvironment.get(environment) ?? new Map<string, Set<string>>()
      importers.set(importer, new Set())
      missingByEnvironment.set(environment, importers)
    },
    observe(importer: string, path: string, environment: object) {
      missingByEnvironment.get(environment)?.get(importer)?.add(path)
      server?.watcher.add(dirname(path))
    },
    close() {
      server?.watcher
        .off('add', changed)
        .off('change', changed)
        .off('unlink', changed)
      server = undefined
      missingByEnvironment.clear()
    },
  }
}
