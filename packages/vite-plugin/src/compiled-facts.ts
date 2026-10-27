import type { GuardFacts, GuardObservation } from '@devup-ui/plugin-utils'
import { compiledGuardFacts } from '@devup-ui/wasm'

interface SerializedGuardFacts {
  readonly references: GuardFacts['references']
  readonly exports: readonly (readonly [
    string,
    Exclude<ReturnType<GuardFacts['exports']['get']>, undefined> | null,
  ])[]
  readonly stars: readonly string[]
  readonly observation?: {
    readonly defaultCallable: boolean
    readonly unsupportedRequests: readonly string[]
    readonly importOnlyRequests?: readonly string[]
  }
}

export function sourceFacts(
  code: string,
  observe?: (value: GuardObservation) => void,
): GuardFacts {
  const facts: SerializedGuardFacts = JSON.parse(
    compiledGuardFacts(code, observe !== undefined),
  )
  if (observe && facts.observation)
    observe({
      defaultCallable: facts.observation.defaultCallable,
      unsupportedRequests: new Set(facts.observation.unsupportedRequests),
      ...(facts.observation.importOnlyRequests
        ? { importOnlyRequests: new Set(facts.observation.importOnlyRequests) }
        : {}),
    })
  return {
    references: facts.references,
    exports: new Map(
      facts.exports.map(([name, origin]) => [name, origin ?? undefined]),
    ),
    stars: facts.stars,
  }
}
