import {
  createCompileTimeClassifier,
  isCompileTimeAlias,
  isSelectedSource,
  UntransformedSourceError,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'
import type { Plugin } from 'vite'

import type { DeferredDefaults } from './deferred-defaults'
import { FinalModules } from './final-modules'
import { ResolutionObservations } from './resolution-observations'

export function createCompiledGuard(options: {
  readonly package: string
  readonly mdxExtensions: readonly string[]
  readonly importAliases: WasmImportAliases
  readonly extractCss: boolean
  readonly deferred?: DeferredDefaults
}): Plugin {
  const environments = new WeakMap<
    object,
    { readonly resolutions: ResolutionObservations; valid: boolean }
  >()
  const classify = createCompileTimeClassifier(options.package)
  const plugin: Plugin = {
    name: 'devup-ui:compiled-source-guard',
    sharedDuringBuild: true,
    apply: 'build',
    options(input) {
      if (
        options.deferred ||
        !this.environment ||
        !Array.isArray(input.plugins)
      )
        return
      const resolutions = new ResolutionObservations()
      environments.set(this.environment, { resolutions, valid: false })
      return { ...input, plugins: [resolutions.plugin, ...input.plugins] }
    },
    buildStart(input) {
      const state = environments.get(this.environment ?? plugin)
      if (!state) return
      state.resolutions.clear()
      state.valid = input.plugins[0] === state.resolutions.plugin
    },
    buildEnd(error) {
      if (error || !options.extractCss) return
      const environment =
        this.environment ?? options.deferred?.fallback ?? plugin
      const modules = new FinalModules(this)
      const state = environments.get(environment)
      function target(
        importer: string,
        ref: {
          readonly request: string
          readonly kind?: 'require-call'
        },
      ) {
        const edge = { importer, ...ref }
        const resolved = options.deferred
          ? options.deferred.resolution.edge(environment, edge)
          : state?.valid
            ? state.resolutions.edge(edge)
            : undefined
        return resolved && modules.get(importer)?.importedIds.includes(resolved)
          ? resolved
          : undefined
      }
      function origins(
        id: string,
        ids: readonly string[],
        seen: ReadonlySet<string> = new Set(),
      ): Map<string, boolean> {
        const key = JSON.stringify([id, ids])
        if (seen.has(key)) return new Map()
        if (classify(id, ids)) return new Map([[key, true]])
        const module = modules.get(id)
        if (!module || !ids.length) return new Map()
        const next = new Set([...seen, key])
        const direct = module.facts.exports.get(ids[0])
        if (module.facts.exports.has(ids[0])) {
          const resolved = direct && target(id, direct)
          return resolved && direct
            ? origins(resolved, [...direct.ids, ...ids.slice(1)], next)
            : new Map([[key, false]])
        }
        return new Map(
          (ids[0] === 'default' ? [] : module.facts.stars).flatMap(
            (request) => {
              const resolved = target(id, { request })
              return resolved ? [...origins(resolved, ids, next)] : []
            },
          ),
        )
      }
      for (const id of this.getModuleIds()) {
        if (
          id.startsWith('\0') ||
          isSelectedSource(id.split('?')[0], options.mdxExtensions)
        )
          continue
        const module = modules.get(id)
        if (!module) continue
        for (const ref of module.facts.references) {
          const resolved = target(id, ref)
          const targets = resolved
            ? origins(resolved, ref.ids)
            : new Map<string, boolean>()
          if (
            !isCompileTimeAlias(
              { ...ref, line: 1, column: 1 },
              options.importAliases,
            ) &&
            (targets.size !== 1 || ![...targets.values()][0])
          )
            continue
          const prefix = module.code.slice(0, ref.pos)
          const reference = {
            ...ref,
            line: prefix.split('\n').length,
            column: prefix.length - prefix.lastIndexOf('\n'),
          }
          this.error({
            id,
            pos: ref.pos,
            message: new UntransformedSourceError(
              id,
              reference,
              isCompileTimeAlias(reference, options.importAliases),
            ).message,
          })
        }
      }
    },
  }
  return plugin
}
