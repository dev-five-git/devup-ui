import type { GuardFacts, GuardObservation } from '@devup-ui/plugin-utils'

import { sourceFacts } from './compiled-facts'

export interface FinalModule {
  readonly id: string
  readonly code: string
  readonly facts: GuardFacts
  readonly observation: GuardObservation | undefined
  readonly importedIds: readonly string[]
}

export interface FinalModuleGraph {
  getModuleInfo(id: string): {
    readonly code: string | null
    readonly importedIds: readonly string[]
  } | null
}

/** One final verification's demand-driven source reads, never a watch cache. */
export class FinalModules {
  private readonly modules = new Map<string, FinalModule | undefined>()

  constructor(private readonly graph: FinalModuleGraph) {}

  get(id: string): FinalModule | undefined {
    if (this.modules.has(id)) return this.modules.get(id)
    const info = this.graph.getModuleInfo(id)
    const code = info?.code
    if (!info || !code) {
      this.modules.set(id, undefined)
      return
    }
    let observation: GuardObservation | undefined
    const facts = sourceFacts(code, (value) => {
      observation = value
    })
    const module = {
      id,
      code,
      importedIds: info.importedIds,
      facts,
      observation,
    }
    this.modules.set(id, module)
    return module
  }
}
