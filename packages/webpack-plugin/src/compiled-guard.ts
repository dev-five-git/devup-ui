import {
  createCompileTimeClassifier,
  createDependencyGuard,
  decodeGuardFacts,
  type DependencyGuardOptions,
  type GuardFacts,
  isCompileTimeAlias,
  isSelectedSource,
  UntransformedSourceError,
} from '@devup-ui/plugin-utils'
import { compiledGuardFacts } from '@devup-ui/wasm'
import type { Compiler, Dependency, Module } from 'webpack'

export function registerCompiledGuard(
  compiler: Compiler,
  options: DependencyGuardOptions,
) {
  const check = createDependencyGuard(options)
  const classify = createCompileTimeClassifier(options.package)
  compiler.hooks.thisCompilation.tap(
    'DevupUICompiledSourceGuard',
    (compilation) => {
      compilation.hooks.finishModules.tap(
        'DevupUICompiledSourceGuard',
        (modules) => {
          const roots = [...modules].filter(
            (module) =>
              module.type.startsWith('javascript') &&
              module.getNumberOfErrors() === 0 &&
              'resource' in module &&
              typeof module.resource === 'string' &&
              !isSelectedSource(
                module.resource.split('?')[0],
                options.mdxExtensions,
              ),
          )
          const facts = new Map<
            Module,
            { readonly code: string; readonly facts: GuardFacts } | undefined
          >()
          function source(module: Module) {
            if (facts.has(module)) return facts.get(module)
            const code =
              module.type.startsWith('javascript') &&
              module.getNumberOfErrors() === 0
                ? module.originalSource()?.source().toString()
                : undefined
            const value =
              code === undefined
                ? undefined
                : {
                    code,
                    facts: decodeGuardFacts(compiledGuardFacts(code, false)),
                  }
            facts.set(module, value)
            return value
          }
          const dependencies = new Map<unknown, Dependency>()
          for (const module of roots)
            for (const dependency of module.dependencies)
              dependencies.set(dependency, dependency)
          const errors = check(roots, {
            graph: compilation.moduleGraph,
            target(dependency) {
              const actual = dependencies.get(dependency)
              const module = actual && compilation.moduleGraph.getModule(actual)
              return module &&
                'resource' in module &&
                typeof module.resource === 'string'
                ? module.resource
                : undefined
            },
            forwarded(dependency, ids) {
              const actual = dependencies.get(dependency)
              let module = actual && compilation.moduleGraph.getModule(actual)
              let names = ids
              const seen = new Set<unknown>()
              while (module && names.length && !seen.has(module)) {
                seen.add(module)
                if (
                  'resource' in module &&
                  typeof module.resource === 'string' &&
                  classify(module.resource, names)
                )
                  return true
                const target = compilation.moduleGraph
                  .getExportsInfo(module)
                  .getExportInfo(names[0])
                  .getTarget(compilation.moduleGraph)
                module = target?.module
                names = target?.export
                  ? [...target.export, ...names.slice(1)]
                  : []
              }
              return false
            },
          })
          function resolved(module: Module, request: string) {
            const dependency = module.dependencies.find(
              (item) => 'request' in item && item.request === request,
            )
            return dependency
              ? compilation.moduleGraph.getModule(dependency)
              : undefined
          }
          function origins(
            module: Module,
            ids: readonly string[],
            seen: ReadonlySet<Module> = new Set(),
          ): Map<string, boolean> {
            if (seen.has(module)) return new Map()
            const key = JSON.stringify([module.identifier(), ids])
            if (
              'resource' in module &&
              typeof module.resource === 'string' &&
              classify(module.resource, ids)
            )
              return new Map([[key, true]])
            const symbols = source(module)?.facts
            if (!symbols || !ids.length) return new Map()
            const next = new Set([...seen, module])
            const direct = symbols.exports.get(ids[0])
            if (symbols.exports.has(ids[0])) {
              const target = direct && resolved(module, direct.request)
              return target && direct
                ? origins(target, [...direct.ids, ...ids.slice(1)], next)
                : new Map([[key, false]])
            }
            if (ids[0] === 'default') return new Map()
            return new Map(
              symbols.stars.flatMap((request) => {
                const target = resolved(module, request)
                return target ? [...origins(target, ids, next)] : []
              }),
            )
          }
          for (const module of roots) {
            if (
              !('resource' in module) ||
              typeof module.resource !== 'string' ||
              errors.some((error) => error.filename === module.resource)
            )
              continue
            const observed = source(module)
            if (!observed) continue
            for (const ref of observed.facts.references) {
              const target = resolved(module, ref.request)
              const targets = target
                ? origins(target, ref.ids)
                : new Map<string, boolean>()
              const prefix = observed.code.slice(0, ref.pos)
              const reference = {
                ...ref,
                line: prefix.split('\n').length,
                column: prefix.length - prefix.lastIndexOf('\n'),
              }
              if (
                isCompileTimeAlias(reference, options.importAliases) ||
                (targets.size === 1 && [...targets.values()][0])
              ) {
                errors.push(
                  new UntransformedSourceError(
                    module.resource,
                    reference,
                    isCompileTimeAlias(reference, options.importAliases),
                  ),
                )
                break
              }
            }
          }
          compilation.errors.push(...errors)
        },
      )
    },
  )
}
