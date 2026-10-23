import type { BuildOptions } from 'vite'

import { inspectImports } from './aggregate-imports'

interface ClosureModule {
  readonly code: string | null
  readonly importedIds: readonly string[]
  readonly dynamicallyImportedIds: readonly string[]
  readonly isExternal?: boolean
  readonly isEntry?: boolean
}

export interface CssClosureContext {
  getModuleIds(): IterableIterator<string>
  getModuleInfo(id: string): ClosureModule | null
  load(options: {
    id: string
    resolveDependencies: boolean
  }): Promise<ClosureModule | null>
  resolve(
    id: string,
    importer?: string,
  ): Promise<{
    readonly id: string
    readonly external: boolean | string
  } | null>
  parse(code: string): unknown
}

interface Preparation {
  readonly entries: readonly string[]
  promise?: Promise<void>
  completed?: { readonly ids: ReadonlySet<string>; readonly asset: string }
}

export class AggregateCssError extends Error {
  readonly file: string
  readonly line: number
  readonly column: number

  constructor(
    location:
      | string
      | {
          readonly file: string
          readonly line: number
          readonly column: number
        },
    readonly asset: string,
    reason: string,
  ) {
    const { file, line, column } =
      typeof location === 'string'
        ? { file: location, line: 1, column: 1 }
        : location
    super(
      `${file}:${line}:${column}: [devup-ui] aggregate CSS asset ${asset} cannot be completed at build time: ${reason}; use an application build with build.lib=false and build.cssCodeSplit=true, or precompile the responsible module and publish its CSS as a separate imported asset`,
    )
    this.file = file
    this.line = line
    this.column = column
    this.name = 'AggregateCssError'
  }
}

// Vite processes these requests as stylesheets, not JS graph vertices. Awaiting
// their loads would wait on this very barrier (including CSS @import cycles).
const CSS_REQUEST =
  /\.(?:css|less|sass|scss|styl|stylus|pcss|postcss|sss)(?:$|\?)/i

export function createAggregateCssPreparation(
  deadline: (expire: () => void) => () => void = (expire) => {
    const timer = setTimeout(expire, 30_000)
    return () => clearTimeout(timer)
  },
) {
  const environments = new WeakMap<object, Preparation>()
  return {
    start(
      environment: object,
      input: string | readonly string[] | Record<string, string> | undefined,
      build: Pick<BuildOptions, 'lib' | 'cssCodeSplit'>,
    ) {
      environments.delete(environment)
      if (!build.lib && build.cssCodeSplit !== false) return
      const entries =
        typeof input === 'string' ? [input] : input ? Object.values(input) : []
      environments.set(environment, { entries })
    },
    prepare(environment: object, context: CssClosureContext, asset: string) {
      const state = environments.get(environment)
      if (!state) return
      state.promise ??= (async () => {
        const visited = new Set<string>()
        const entries = new Set(state.entries)
        for (const id of context.getModuleIds()) {
          if (context.getModuleInfo(id)?.isEntry) entries.add(id)
        }
        async function resolveModule(id: string, importer?: string) {
          try {
            return await context.resolve(id, importer)
          } catch (cause) {
            throw new AggregateCssError(
              importer ?? id,
              asset,
              `public resolution refused ${id}: ${cause instanceof Error ? cause.message : String(cause)}`,
            )
          }
        }
        async function walk(id: string, importer: string): Promise<void> {
          if (visited.has(id)) return
          visited.add(id)
          const known = context.getModuleInfo(id)
          if (known?.isExternal) return
          if (!known || known.code === null) {
            const resolved = await resolveModule(id, importer)
            if (!resolved && !known)
              throw new AggregateCssError(
                importer,
                asset,
                `public resolution refused ${id}`,
              )
            if (resolved?.external) return
            if (resolved && resolved.id !== id) {
              if (visited.has(resolved.id))
                throw new AggregateCssError(
                  importer,
                  asset,
                  `public resolution formed an ID cycle at ${id}`,
                )
              return walk(resolved.id, importer)
            }
          }
          if (CSS_REQUEST.test(id)) return
          let code = ''
          function fail(reason: string, position?: number): never {
            const lines = code.slice(0, position).split('\n')
            const location =
              position === undefined
                ? id
                : {
                    file: id,
                    line: lines.length,
                    column: (lines.at(-1)?.length ?? 0) + 1,
                  }
            throw new AggregateCssError(location, asset, reason)
          }
          let info: ClosureModule | null
          try {
            info = await new Promise<ClosureModule | null>(
              (resolve, reject) => {
                const cancel = deadline(() =>
                  reject(
                    new AggregateCssError(
                      id,
                      asset,
                      'public load did not settle within 30 seconds; a source transform awaiting generated CSS creates a preparation cycle',
                    ),
                  ),
                )
                Promise.resolve()
                  .then(() => context.load({ id, resolveDependencies: false }))
                  .then(resolve, reject)
                  .finally(cancel)
              },
            )
          } catch (cause) {
            fail(
              `public load refused the transformed module: ${cause instanceof Error ? cause.message : String(cause)}`,
            )
          }
          if (
            !info ||
            typeof info.code !== 'string' ||
            !Array.isArray(info.importedIds) ||
            !Array.isArray(info.dynamicallyImportedIds)
          ) {
            fail(
              'public load returned null or incomplete transformed module information',
            )
          }
          const dependencies = [
            ...info.importedIds,
            ...info.dynamicallyImportedIds,
          ]
          code = info.code
          let hasImports: boolean
          try {
            const ast = context.parse(code)
            if (
              !ast ||
              typeof ast !== 'object' ||
              !('type' in ast) ||
              ast.type !== 'Program'
            )
              fail('public parser returned incomplete module information')
            hasImports = inspectImports(ast, fail)
          } catch (cause) {
            if (cause instanceof AggregateCssError) throw cause
            fail(
              `public parser refused the transformed module: ${cause instanceof Error ? cause.message : String(cause)}`,
            )
          }
          if (hasImports && dependencies.length === 0)
            fail('public load returned an unresolved import graph')
          for (const dependency of dependencies) await walk(dependency, id)
        }
        if (entries.size === 0)
          throw new AggregateCssError(
            asset,
            asset,
            'the public build input list is empty',
          )
        for (const entry of entries) {
          const resolved = await resolveModule(entry)
          if (!resolved || resolved.external)
            throw new AggregateCssError(
              entry,
              asset,
              'the public build entry could not be resolved to an internal module',
            )
          await walk(resolved.id, entry)
        }
        state.completed = { ids: visited, asset }
      })()
      return state.promise
    },
    observe(environment: object, id: string) {
      const completed = environments.get(environment)?.completed
      if (completed && !completed.ids.has(id)) {
        throw new AggregateCssError(
          id,
          completed.asset,
          'a plugin introduced a module after the aggregate closure completed; expose its entry/imports before the first generated CSS load',
        )
      }
    },
    checkAsset(
      environment: object,
      asset: { readonly name?: string; readonly fileName: string },
    ) {
      const state = environments.get(environment)
      if (state && /^devup-ui(-\d+)?\.css$/.test(asset.name ?? '')) {
        throw new AggregateCssError(
          state.entries[0] ?? asset.fileName,
          asset.fileName,
          'the native split library emitted a standalone generated stylesheet that the shared-sheet output contract rewrites after optimization; use build.cssCodeSplit=false for this library',
        )
      }
    },
  }
}
