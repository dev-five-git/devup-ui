import { parseAst } from 'vite'

import {
  createAggregateCssPreparation,
  type CssClosureContext,
} from '../aggregate-css'

type Module = NonNullable<ReturnType<CssClosureContext['getModuleInfo']>>

export function module(
  code = '',
  importedIds: readonly string[] = [],
  dynamicallyImportedIds: readonly string[] = [],
): Module {
  return { code, importedIds, dynamicallyImportedIds }
}

export function fixture(
  entries:
    string | readonly string[] | Record<string, string> | undefined = 'entry',
) {
  const environment = {}
  const preparation = createAggregateCssPreparation()
  const modules = new Map<string, Module>([['entry', module()]])
  const cached = new Map<string, Module>()
  const loaded = new Set<string>()
  const context: CssClosureContext = {
    getModuleIds() {
      return cached.keys()
    },
    getModuleInfo(id) {
      return cached.get(id) ?? null
    },
    async resolve(id) {
      return { id, external: false }
    },
    async load({ id }) {
      loaded.add(id)
      return modules.get(id) ?? null
    },
    parse(code) {
      return parseAst(code)
    },
  }
  preparation.start(environment, entries, { lib: { entry: 'entry' } })
  return { environment, preparation, context, modules, cached, loaded }
}
