import {
  decodeGuardFacts,
  type GuardFacts,
  type GuardObservation,
} from '@devup-ui/plugin-utils'
import { compiledGuardFacts } from '@devup-ui/wasm'

export function sourceFacts(
  code: string,
  observe?: (value: GuardObservation) => void,
): GuardFacts {
  return decodeGuardFacts(
    compiledGuardFacts(code, observe !== undefined),
    observe,
  )
}
