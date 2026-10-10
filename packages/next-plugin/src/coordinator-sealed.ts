import {
  extractRequest,
  locatedError,
  type RebuildPlan,
} from './coordinator-engine'
import type { ExtractRequest } from './coordinator-http'
import {
  type CoordinatorSnapshot,
  type JsonObject,
  restoreCoordinatorState,
} from './state'
import type { DevupWasm } from './wasm'

type SealedPlan = Pick<
  RebuildPlan,
  'live' | 'createEngine' | 'configure' | 'settings'
>

function sameCss(live: DevupWasm, candidate: DevupWasm): boolean {
  const liveMap: JsonObject = JSON.parse(live.exportFileMap())
  const candidateMap: JsonObject = JSON.parse(candidate.exportFileMap())
  const buckets = new Set([
    undefined,
    ...Object.values(liveMap).filter((value) => typeof value === 'number'),
    ...Object.values(candidateMap).filter((value) => typeof value === 'number'),
  ])
  // New numbers used to fall back to base CSS; compare them on both engines too.
  for (const fileNum of buckets) {
    for (const importMainCss of [false, true]) {
      if (
        live.getCss(fileNum, importMainCss) !==
        candidate.getCss(fileNum, importMainCss)
      )
        return false
    }
  }
  return true
}

export function extractSealed(
  plan: SealedPlan,
  snapshot: CoordinatorSnapshot,
  request: ExtractRequest,
) {
  try {
    const candidate = plan.createEngine()
    if (candidate === plan.live) {
      throw locatedError(
        request.filename,
        'validate production re-extraction',
        'createEngine returned the engine in service',
        'createEngine must return a new, isolated engine on every call.',
      )
    }
    // Restore the actual sheet, not a replay: ordinary atoms and keyframes retain history.
    restoreCoordinatorState(candidate, snapshot)
    // Sheet exports omit theme, so frozen configuration must follow restoration.
    plan.configure(candidate)
    if (!sameCss(plan.live, candidate)) {
      throw locatedError(
        request.filename,
        'validate production re-extraction',
        'the restored candidate does not render the live stylesheet',
        'createEngine and configureWasm must restore an isolated engine with the same frozen production configuration.',
      )
    }
    const output = extractRequest(candidate, plan.settings, request)
    if (!sameCss(plan.live, candidate)) {
      throw locatedError(
        request.filename,
        'change styles after the production stylesheet was served',
        'its extraction produced CSS the served stylesheet lacks',
        'include it in expectedBaseFiles (or the prewarm) so its styles exist before CSS is served.',
      )
    }
    return { engine: candidate, output: { ...output, updatedBaseStyle: false } }
  } catch (error) {
    throw locatedError(
      request.filename,
      'validate production re-extraction',
      error,
      'fix the reported source or the isolated engine factory; the served stylesheet must remain unchanged.',
    )
  }
}
