import { readFileSync } from 'node:fs'

import {
  applyMarkerEdits,
  ComponentMarkerLedger,
  markerEditMap,
  markerEdits,
  markerLikeTokens,
  markerWarning,
  unscannedCause,
} from '@devup-ui/plugin-utils'

interface MarkerOutput {
  readonly givenMarkers?: readonly string[]
  readonly selectedMarkers?: readonly string[]
}

interface RenderedChunk {
  /** The rendered code of each module in the chunk, by id */
  readonly modules: Readonly<Record<string, { readonly code: string | null }>>
}

function readSource(file: string): string | undefined {
  try {
    return readFileSync(file, 'utf-8')
  } catch {
    return undefined
  }
}

/**
 * Component-selector markers in a Vite build: what the scan and the
 * transforms report, and the chunks rewritten without the markers no module
 * selects once every module is transformed
 */
export function createViteMarkers() {
  const ledger = new ComponentMarkerLedger()
  let warned = false
  let plan: ReturnType<ComponentMarkerLedger['plan']> | undefined
  return {
    /** A new build begins */
    reset() {
      ledger.clear()
      plan = undefined
      warned = false
    },
    /**
     * An environment's build begins: its chunks follow the plan made once its
     * own modules are transformed
     */
    scanned(selections: readonly string[] | undefined, ids: readonly string[]) {
      ledger.scanned(selections, ids)
      plan = undefined
      warned = false
    },
    /** What the transform of module `file` read and gave */
    record(file: string, input: string, output: MarkerOutput) {
      const id = file.replaceAll('\\', '/')
      const selectedMarkers = output.selectedMarkers ?? []
      ledger.record({
        id,
        input,
        givenMarkers: output.givenMarkers ?? [],
        selectedMarkers,
        cause: selectedMarkers.length
          ? unscannedCause(id, input, readSource(file))
          : undefined,
      })
    },
    /**
     * `code` without the markers no module selects, with its map; `null` when
     * nothing changes. `context` hears once why a build keeps them.
     */
    renderChunk(
      code: string,
      chunk: RenderedChunk,
      context: { warn(message: string): void },
    ) {
      plan ??= ledger.plan()
      if ('keep' in plan) {
        const warning = markerWarning(plan.keep)
        if (warning && !warned) context.warn(warning)
        warned = true
        return null
      }
      const foreign = new Set<string>()
      for (const [id, module] of Object.entries(chunk.modules))
        if (!ledger.recorded(id.replaceAll('\\', '/')) && module.code)
          for (const token of markerLikeTokens(module.code)) foreign.add(token)
      const strip = new Set(
        [...plan.strip].filter((marker) => !foreign.has(marker)),
      )
      const edits = markerEdits(code, strip)
      if (!edits.length) return null
      return {
        code: applyMarkerEdits(code, edits),
        map: markerEditMap(code, edits),
      }
    },
  }
}
