import type { SourcePlan } from './plan'
import type { AppContext } from './session'
import {
  type DevupWasm,
  type ModuleResolverSettings,
  withModuleResolver,
} from './wasm'

export interface EngineSettings {
  /** The resolved `theme` of the devup config, `{}` when there is none */
  theme: object
  plan: Pick<SourcePlan, 'canonicalMap' | 'fileRoutes' | 'atomThreshold'>
  readonly resolver?: ModuleResolverSettings
}

/**
 * The function that gives an engine this app's configuration. It sets every
 * option, the disabled and default ones too, so nothing a previous app (or a
 * previous state of this one) left in an engine can survive it.
 */
export function createEngineConfigurer(
  context: AppContext,
  { theme, plan, resolver }: EngineSettings,
): (engine: DevupWasm) => void {
  return (engine) => {
    engine.setDebug(context.debug)
    engine.setPrefix(context.prefix)
    engine.registerShorthands(context.shorthands)
    // importSheet replaces the whole sheet, theme included, so the theme is
    // registered by every configuration, after any state import.
    engine.registerTheme(theme)
    engine.importCanonicalMap(plan.canonicalMap)
    engine.importFileRoutes(plan.fileRoutes)
    engine.setAtomHoist(plan.atomThreshold)
    withModuleResolver(engine, context.root, resolver)
  }
}
