import { locatedError } from './coordinator-engine'
import type { ExtractRequest } from './coordinator-http'
import type { PreparedSourceGeneration } from './coordinator-options'
import type { CoordinatorInput } from './state'

export function immutableInput(input: CoordinatorInput): CoordinatorInput {
  return Object.freeze({
    ...input,
    dependencies: Object.freeze([...input.dependencies]),
    stamps: Object.freeze({ ...input.stamps }),
  })
}

/** Copy provider data once; resolver/configuration must capture the same immutable bytes. */
export function immutableGeneration(
  generation: PreparedSourceGeneration,
): PreparedSourceGeneration {
  return Object.freeze({
    configureWasm: generation.configureWasm,
    sources: Object.freeze(
      generation.sources.map(({ input, evidence }) =>
        Object.freeze({
          input: immutableInput(input),
          evidence: Object.freeze({
            ...evidence,
            fileFingerprints: Object.freeze({ ...evidence.fileFingerprints }),
            contextFingerprints: Object.freeze({
              ...evidence.contextFingerprints,
            }),
            missingDependencies: Object.freeze([
              ...evidence.missingDependencies,
            ]),
          }),
        }),
      ),
    ),
  })
}

export function preparedInput(
  generation: PreparedSourceGeneration | undefined,
  request: ExtractRequest,
): CoordinatorInput | undefined {
  const input = generation?.sources.find(
    (source) => source.input.filename === request.filename,
  )?.input
  if (input !== undefined && input.source !== request.code) {
    throw locatedError(
      request.filename,
      'accept compiled source from a different prepared generation',
      'the loader bytes differ from the current compiler-owned input',
      'refresh the native compiler input and retry; stale compiled bytes cannot replace prepared sources.',
    )
  }
  return input
}
