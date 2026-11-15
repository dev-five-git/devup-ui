import {
  createModuleResolver,
  ProductionNumberingError,
} from '@devup-ui/plugin-utils'
import {
  importCanonicalMap,
  importFileRoutes,
  registerShorthands,
  registerTheme,
  reserveClassSlots,
  seedFileMap,
  setAtomHoist,
  setDebug,
  setModuleResolver,
  setPrefix,
} from '@devup-ui/wasm'

import type { ProductionGeneration } from './production-engine'
import type { ProductionOptions } from './production-manifest'

export function configureProduction(
  generation: ProductionGeneration,
  options: ProductionOptions,
  key: string,
): void {
  const manifest = generation.manifest
  if (manifest === undefined)
    throw new ProductionNumberingError('unseeded', { context: key })
  const runtime = generation.runtime(key)
  registerTheme(manifest.theme)
  registerShorthands(options.shorthands ?? {})
  setDebug(options.debug)
  setPrefix(options.prefix ?? null)
  importCanonicalMap(runtime.plan.canonical)
  importFileRoutes(runtime.plan.routes)
  setAtomHoist(runtime.plan.threshold ?? null)
  setModuleResolver(undefined)
  generation.numbering?.seed({ seedFileMap, importCanonicalMap }, key)
  if (generation.numbering !== undefined && !runtime.reserved) {
    setModuleResolver(
      createModuleResolver({
        cwd: manifest.root,
        conditions: runtime.plan.conditions,
        toId: (path) => path.replaceAll('\\', '/'),
      }),
    )
    reserveClassSlots(
      runtime.plan.slots,
      options.package,
      options.singleCss,
      true,
      false,
      options.importAliases,
    )
    runtime.reserved = true
    setModuleResolver(undefined)
  }
  generation.current = key
}
