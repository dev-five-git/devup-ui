import { rm } from 'node:fs/promises'
import { join } from 'node:path'

import type { Plugin } from 'vite'

type Sheets = Map<number | null, string>

/**
 * The environments of one app build (client, ssr, ...) share one engine, and
 * Vite builds them one after another. A sheet an environment emits holds the
 * styles extracted so far, so a style that only a later environment extracts
 * (a module only the server renders) never reaches it, and what each
 * environment holds depends on the order they build in.
 *
 * This records what each environment emitted of every devup-ui sheet, and
 * once every environment is built, builds again each one whose sheets have
 * changed since. Every module has been extracted by then, so the sheets of
 * that build are final: each environment ends up holding every style of the
 * sheets it emits, whatever the order.
 */
export function createEnvironmentSheets(
  read: (fileNum: number | null) => string,
) {
  const emitted = new Map<string, Sheets>()
  const written = new Map<string, string[]>()
  const changed = (name: string) =>
    [...(emitted.get(name) ?? [])].some(
      ([fileNum, css]) => read(fileNum) !== css,
    )
  const plugin: Plugin = {
    name: 'devup-ui:complete-environment-sheets',
    sharedDuringBuild: true,
    apply: 'build',
    writeBundle(options, bundle) {
      const dir = options.dir
      if (!dir || !this.environment) return
      written.set(
        this.environment.name,
        Object.keys(bundle).map((file) => join(dir, file)),
      )
    },
    buildApp: {
      order: 'post',
      async handler(builder) {
        const environments = Object.values(builder.environments)
        // Vite builds every environment after these hooks unless one is built
        // already; build them here so that the sheets are checked after all
        if (environments.every((environment) => !environment.isBuilt))
          for (const environment of environments)
            await builder.build(environment)
        // A build again extracts the modules it extracted before, adding no
        // style, so a single pass leaves every sheet final
        for (const environment of environments.filter(({ name }) =>
          changed(name),
        )) {
          // Its output is written again in full, under names that can change
          await Promise.all(
            (written.get(environment.name) ?? []).map((file) =>
              rm(file, { force: true }),
            ),
          )
          await builder.build(environment)
        }
      },
    },
  }
  return {
    /** The sheets one build of `environment` emits, recorded from now on. */
    emit(environment: string): Sheets {
      const sheets: Sheets = new Map()
      emitted.set(environment, sheets)
      return sheets
    },
    plugin,
  }
}
