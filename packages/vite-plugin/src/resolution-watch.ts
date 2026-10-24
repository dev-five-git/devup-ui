import { existsSync } from 'node:fs'
import { basename, dirname, join, resolve } from 'node:path'

import { resolutionWatchPath, SOURCE_FILE_RE } from '@devup-ui/plugin-utils'
import type { ViteDevServer } from 'vite'

export function createMissingInputWatch(
  invalidate?: (importer: string) => void,
) {
  const missingByEnvironment = new Map<object, Map<string, Set<string>>>()
  let server: ViteDevServer | undefined
  const changed = (file: string) => {
    const path = file.replaceAll('\\', '/')
    let affected = false
    for (const environment of Object.values(server?.environments ?? {})) {
      for (const [importer, inputs] of missingByEnvironment.get(environment) ??
        []) {
        if (!inputs.has(path)) continue
        invalidate?.(importer)
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
    observeFile(importer: string, path: string, environment: object) {
      missingByEnvironment.get(environment)?.get(importer)?.add(path)
    },
    observe(importer: string, path: string, environment: object) {
      const inputs = missingByEnvironment.get(environment)?.get(importer)
      inputs?.add(path)
      if (!server) return
      if (!SOURCE_FILE_RE.test(path)) {
        server.watcher.add(
          basename(path) === 'package.json' ? path : dirname(path),
        )
        return
      }
      const physical = resolutionWatchPath(path, false).replaceAll('\\', '/')
      inputs?.add(physical)
      let directory = dirname(path)
      while (!existsSync(directory) && dirname(directory) !== directory)
        directory = dirname(directory)
      const target = resolve(resolutionWatchPath(directory, false))
      if (
        dirname(target) === target ||
        target === resolve(resolutionWatchPath(server.config.root, false)) ||
        target ===
          resolve(
            resolutionWatchPath(
              join(server.config.root, 'node_modules'),
              false,
            ),
          ) ||
        basename(target) === 'node_modules'
      )
        return
      server.watcher.add(directory)
      if (target !== resolve(directory)) server.watcher.add(target)
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
