import {
  BuildGeneration,
  type GenerationEngine,
  nextNonphysicalIdList,
  type ProductionNumbering,
  ProductionNumberingError,
} from '@devup-ui/plugin-utils'
import {
  exportCanonicalMap,
  exportClassMap,
  exportFileMap,
  exportSheet,
  importCanonicalMap,
  importClassMap,
  importFileMap,
  importSheet,
  resetBuildState,
} from '@devup-ui/wasm'

import type { NativePresence, NonphysicalModules } from './nonphysical-modules'
import type {
  prepareProductionManifest,
  ProductionContext,
} from './production-manifest'

interface Runtime {
  readonly plan: ProductionContext
  readonly modules: NonphysicalModules
  learned: readonly string[]
}

export class ProductionGeneration {
  declare readonly owner: BuildGeneration<ProductionState>
  declare readonly contexts: Map<string, Runtime>
  declare readonly participants: Map<string, () => void>
  declare status: 'open' | 'failed' | 'closed'
  declare pending: Promise<void> | undefined
  declare manifest:
    Awaited<ReturnType<typeof prepareProductionManifest>> | undefined
  declare numbering: ProductionNumbering | undefined
  declare current: string | undefined

  constructor() {
    this.owner = new BuildGeneration<ProductionState>()
    this.contexts = new Map<string, Runtime>()
    this.participants = new Map<string, () => void>()
    this.status = 'open'
    this.pending = undefined
    this.manifest = undefined
    this.numbering = undefined
    this.current = undefined
  }

  assertOpen(): void {
    this.owner.acquire(false)()
  }

  runtime(key: string): Runtime {
    const runtime = this.contexts.get(key)
    if (runtime === undefined)
      throw new ProductionNumberingError('invalid-plan', { context: key })
    return runtime
  }

  async finish(
    key: string,
    graph: NativePresence,
    error?: Error,
  ): Promise<void> {
    if (error !== undefined) this.fail()
    if (!this.participants.has(key)) return
    const runtime = this.runtime(key)
    if (this.status !== 'open') {
      runtime.modules.start()
      return
    }
    try {
      const completed = await runtime.modules.finish(graph, runtime.learned)
      this.assertOpen()
      await runtime.plan.store.rewrite(completed.ids, runtime.plan.scan)
      runtime.learned = nextNonphysicalIdList(completed.ids, runtime.plan.scan)
      runtime.modules.publish(completed)
    } catch (cause) {
      this.fail()
      throw cause
    }
  }

  close(key = this.current): void {
    if (key === undefined) return
    this.participants.get(key)?.()
    this.participants.delete(key)
  }

  dispose(): void {
    for (const release of this.participants.values()) release()
    this.participants.clear()
    this.status = 'closed'
    this.owner.dispose()
  }

  fail(): void {
    if (this.status === 'closed') return
    this.status = 'failed'
    this.owner.dispose()
    for (const runtime of this.contexts.values()) runtime.modules.start()
  }
}

export class ProductionLifecycle {
  declare generation: ProductionGeneration

  constructor() {
    this.generation = new ProductionGeneration()
  }

  replace(): void {
    this.generation.dispose()
    this.generation = new ProductionGeneration()
  }

  input(): boolean {
    switch (this.generation.status) {
      case 'open':
        return false
      case 'failed':
        this.replace()
        return true
      case 'closed':
        this.generation.assertOpen()
        return false
    }
  }
}

export interface ProductionState {
  readonly sheet: string
  readonly classes: string
  readonly files: string
  readonly canonical: string
}

export const productionEngine: GenerationEngine<ProductionState> = {
  reset: resetBuildState,
  restore(state) {
    const sheet: unknown = JSON.parse(state.sheet)
    const classes: unknown = JSON.parse(state.classes)
    const files: unknown = JSON.parse(state.files)
    const canonical: unknown = JSON.parse(state.canonical)
    importSheet(sheet)
    importClassMap(classes)
    importFileMap(files)
    importCanonicalMap(canonical)
  },
  capture() {
    return {
      sheet: exportSheet(),
      classes: exportClassMap(),
      files: exportFileMap(),
      canonical: exportCanonicalMap(),
    }
  },
}
