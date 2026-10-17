import {
  compiledFacts,
  createCompileTimeClassifier,
  type GuardFacts,
  isCompileTimeAlias,
  isSelectedSource,
  UntransformedSourceError,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'
import type { Plugin } from 'vite'

export function createCompiledGuard(options: {
  readonly package: string
  readonly mdxExtensions: readonly string[]
  readonly importAliases: WasmImportAliases
  readonly extractCss: boolean
}): Plugin {
  type Module = {
    readonly facts: GuardFacts
    readonly code: string
    readonly targets: ReadonlyMap<string, string>
  }
  const environments = new Map<object, Map<string, Module>>()
  const classify = createCompileTimeClassifier(options.package)
  const targetKey = (ref: {
    readonly request: string
    readonly kind?: 'require-call'
  }) => JSON.stringify([ref.request, ref.kind])
  function environmentModules(environment: object) {
    let modules = environments.get(environment)
    if (!modules) {
      modules = new Map<string, Module>()
      environments.set(environment, modules)
    }
    return modules
  }
  const plugin: Plugin = {
    name: 'devup-ui:compiled-source-guard',
    sharedDuringBuild: true,
    apply: 'build',
    buildStart() {
      environmentModules(this.environment ?? plugin).clear()
    },
    async moduleParsed(info) {
      if (!options.extractCss || !info.code) return
      const facts = compiledFacts(this.parse(info.code))
      const requests = new Map<
        string,
        { readonly request: string; readonly kind?: 'require-call' }
      >(
        [
          ...facts.references,
          ...[...facts.exports.values()].flatMap((ref) => (ref ? [ref] : [])),
          ...facts.stars.map((request) => ({ request })),
        ].map((ref) => [targetKey(ref), ref]),
      )
      const targets = new Map<string, string>()
      for (const [key, ref] of requests) {
        const target = await this.resolve(ref.request, info.id, {
          kind: ref.kind ?? 'import-statement',
        })
        if (target && info.importedIds.includes(target.id))
          targets.set(key, target.id)
      }
      environmentModules(this.environment ?? plugin).set(info.id, {
        facts,
        code: info.code,
        targets,
      })
    },
    buildEnd(error) {
      if (error || !options.extractCss) return
      const modules = environmentModules(this.environment ?? plugin)
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
          const target = direct && module.targets.get(targetKey(direct))
          return target && direct
            ? origins(target, [...direct.ids, ...ids.slice(1)], next)
            : new Map([[key, false]])
        }
        return new Map(
          (ids[0] === 'default' ? [] : module.facts.stars).flatMap(
            (request) => {
              const target = module.targets.get(targetKey({ request }))
              return target ? [...origins(target, ids, next)] : []
            },
          ),
        )
      }
      for (const [id, module] of modules) {
        if (
          id.startsWith('\0') ||
          isSelectedSource(id.split('?')[0], options.mdxExtensions)
        )
          continue
        for (const ref of module.facts.references) {
          const target = module.targets.get(targetKey(ref))
          const targets = target
            ? origins(target, ref.ids)
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
