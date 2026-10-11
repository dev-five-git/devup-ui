import {
  compiledFacts,
  createCompileTimeClassifier,
  createDependencyGuard,
  type DependencyGuardOptions,
  type GuardFacts,
  isCompileTimeAlias,
  isSelectedSource,
  UntransformedSourceError,
} from '@devup-ui/plugin-utils'
import type { Compiler, Dependency, Module } from 'webpack'

interface ScriptParser {
  readonly state: { readonly module: Module }
  readonly hooks: {
    readonly program: {
      tap(
        options: { readonly name: string; readonly stage: number },
        callback: (ast: unknown) => void,
      ): void
    }
  }
}

function isScriptParser(value: object): value is ScriptParser {
  const hooks = 'hooks' in value ? value.hooks : undefined
  const program =
    hooks && typeof hooks === 'object' && 'program' in hooks
      ? hooks.program
      : undefined
  return (
    !!program &&
    typeof program === 'object' &&
    'tap' in program &&
    typeof program.tap === 'function' &&
    'state' in value
  )
}

export function registerCompiledGuard(
  compiler: Compiler,
  options: DependencyGuardOptions,
) {
  const check = createDependencyGuard(options)
  const classify = createCompileTimeClassifier(options.package)
  compiler.hooks.thisCompilation.tap(
    'DevupUICompiledSourceGuard',
    (compilation, { normalModuleFactory }) => {
      const facts = new Map<Module, GuardFacts>()
      for (const type of [
        'javascript/auto',
        'javascript/esm',
        'javascript/dynamic',
      ])
        normalModuleFactory.hooks.parser
          .for(type)
          .tap('DevupUICompiledSourceGuard', (parser) => {
            if (!isScriptParser(parser)) return
            parser.hooks.program.tap(
              { name: 'DevupUICompiledSourceGuard', stage: -10000 },
              (ast) => {
                facts.set(parser.state.module, compiledFacts(ast))
              },
            )
          })
      compilation.hooks.finishModules.tap(
        'DevupUICompiledSourceGuard',
        (modules) => {
          const dependencies = new Map<unknown, Dependency>()
          for (const module of modules)
            for (const dependency of module.dependencies)
              dependencies.set(dependency, dependency)
          const errors = check(modules, {
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
            const symbols = facts.get(module)
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
          for (const module of modules) {
            if (
              !('resource' in module) ||
              typeof module.resource !== 'string' ||
              isSelectedSource(module.resource, options.mdxExtensions) ||
              errors.some((error) => error.filename === module.resource)
            )
              continue
            for (const ref of facts.get(module)?.references ?? []) {
              const target = resolved(module, ref.request)
              const targets = target
                ? origins(target, ref.ids)
                : new Map<string, boolean>()
              const prefix = (
                module.originalSource()?.source().toString() ?? ''
              ).slice(0, ref.pos)
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
