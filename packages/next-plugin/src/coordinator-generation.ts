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
    ...(generation.resolutionInputs === undefined
      ? {}
      : {
          resolutionInputs: Object.freeze(
            generation.resolutionInputs.map((input) =>
              Object.freeze({ ...input }),
            ),
          ),
        }),
    ...(generation.ordinaryInputs === undefined
      ? {}
      : {
          ordinaryInputs: Object.freeze(
            generation.ordinaryInputs.map(immutableInput),
          ),
        }),
    ...(generation.plan === undefined
      ? {}
      : {
          plan: Object.freeze({
            canonicalMap: Object.freeze({ ...generation.plan.canonicalMap }),
            expectedBaseFiles: Object.freeze([
              ...generation.plan.expectedBaseFiles,
            ]),
          }),
        }),
    ...(generation.watchInputs === undefined
      ? {}
      : {
          watchInputs: Object.freeze([...generation.watchInputs]),
        }),
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

export function overlayGeneration(
  inputs: readonly CoordinatorInput[],
  previous: PreparedSourceGeneration | undefined,
  next: PreparedSourceGeneration,
): readonly CoordinatorInput[] {
  const compiled = new Set(next.sources.map(({ input }) => input.filename))
  const formerlyCompiled = new Set(
    previous?.sources.map(({ input }) => input.filename),
  )
  const replacements = new Map<string, CoordinatorInput>()
  for (const input of next.ordinaryInputs ?? []) {
    if (compiled.has(input.filename) || replacements.has(input.filename)) {
      throw locatedError(
        input.filename,
        'adopt ordinary inputs',
        'duplicate or compiled ownership',
        'supply one ordinary input per filename, separate from compiled sources.',
      )
    }
    replacements.set(input.filename, input)
  }
  for (const { input } of next.sources) {
    if (replacements.has(input.filename)) {
      throw locatedError(
        input.filename,
        'adopt compiled inputs',
        'duplicate ownership',
        'supply one compiled input per filename.',
      )
    }
    replacements.set(input.filename, input)
  }
  return [
    ...inputs.filter(
      ({ filename }) =>
        !formerlyCompiled.has(filename) && !replacements.has(filename),
    ),
    ...replacements.values(),
  ]
}

export function assertAdoptedRequest(
  inputs: readonly CoordinatorInput[],
  request: ExtractRequest,
): void {
  const input = inputs.find(({ filename }) => filename === request.filename)
  if (
    input?.source !== request.code ||
    input.resourcePath !== request.resourcePath ||
    input.sourceType !== request.sourceType
  ) {
    throw locatedError(
      request.filename,
      'admit ordinary native bytes',
      'no current generation input matches this request',
      'refresh the disk-first generation; upstream native bytes require the approved owner-held issuer protocol.',
    )
  }
}

export function preparedInput(
  generation: PreparedSourceGeneration | undefined,
  request: ExtractRequest,
): CoordinatorInput | undefined {
  const input = generation?.sources.find(
    (source) => source.input.filename === request.filename,
  )?.input
  if (
    input !== undefined &&
    (input.source !== request.code || input.sourceType !== request.sourceType)
  ) {
    throw locatedError(
      request.filename,
      'accept compiled source from a different prepared generation',
      'the loader bytes differ from the current compiler-owned input',
      'refresh the native compiler input and retry; stale compiled bytes cannot replace prepared sources.',
    )
  }
  return input
}
