import {
  buildEngine,
  type ExtractOutputSnapshot,
  locatedError,
  type RebuildPlan,
} from './coordinator-engine'
import { immutableInput } from './coordinator-generation'
import { orderInputs } from './coordinator-ledger'
import type {
  PreparedSourceGeneration,
  PreparedSources,
  ReplayPreparation,
} from './coordinator-options'

export function createReplayObserver(
  plan: Omit<RebuildPlan, 'inputs' | 'configure'>,
): NonNullable<ReplayPreparation['observeExtraction']> {
  return (inputs, configure) => {
    const ordered = Object.freeze(orderInputs(inputs.map(immutableInput)))
    const outputs = new Map<string, ExtractOutputSnapshot>()
    buildEngine({ ...plan, inputs: ordered, configure, outputs })
    return Object.freeze(
      ordered.map(({ filename }) =>
        Object.freeze({
          filename,
          dependencies: Object.freeze([
            ...(outputs.get(filename)?.dependencies ?? []),
          ]),
        }),
      ),
    )
  }
}

export async function prepareObservedReplay(
  prepared: PreparedSources,
  request: ReplayPreparation,
  plan: Omit<RebuildPlan, 'inputs' | 'configure'>,
): Promise<PreparedSourceGeneration> {
  const observe = createReplayObserver(plan)
  let active = true
  try {
    return await prepared.prepareReplay({
      ...request,
      observeExtraction(inputs, configure) {
        request.signal.throwIfAborted()
        if (!active) {
          throw locatedError(
            'devup-ui.css',
            'observe an expired replay',
            'preparation already settled',
            'observe complete inputs only during the owning preparation call.',
          )
        }
        return observe(inputs, configure)
      },
    })
  } finally {
    active = false
  }
}
