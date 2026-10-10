import {
  compareMdxBindingValues,
  MdxBindingDifference,
  snapshotMdxBindingValue,
} from './mdx-binding-value'

export type MdxNativeLoaderFacts = {
  readonly resolvedPath: string
  readonly packageVersion: string
  readonly verifiedFileHash: string
  readonly options: unknown
}
export type MdxBindingEvaluation = {
  readonly evaluationID: string
  readonly configFile: string
  readonly projectDir: string
  readonly phase: string
  readonly isolateID: string
}
export type MdxBindingDelivery = {
  readonly sourceMap: boolean
  readonly layer: string | undefined
  readonly isServer: boolean
  readonly compilerName: string
}
export type MdxBindingInput = {
  readonly owner: MdxBindingOwner
  readonly loaders: readonly MdxNativeLoaderFacts[]
  readonly evaluation: MdxBindingEvaluation
  readonly delivery: MdxBindingDelivery
}

/** Capture original native/config options before wrapper tuple mutation, not its artifacts. */
export class MdxBindingReceipt {
  readonly owner: MdxBindingOwner
  readonly loaders: readonly MdxNativeLoaderFacts[]
  readonly evaluation: MdxBindingEvaluation
  readonly delivery: MdxBindingDelivery
  readonly #snapshot: ReturnType<typeof snapshotMdxBindingValue>
  constructor(input: MdxBindingInput) {
    this.owner = Object.freeze({ ...input.owner })
    this.loaders = Object.freeze(
      input.loaders.map(
        ({ resolvedPath, packageVersion, verifiedFileHash, options }) =>
          Object.freeze({
            resolvedPath,
            packageVersion,
            verifiedFileHash,
            options,
          }),
      ),
    )
    this.evaluation = Object.freeze({ ...input.evaluation })
    this.delivery = Object.freeze({ ...input.delivery })
    // Opaque native option values remain live; only comparison shells are detached.
    this.#snapshot = snapshotMdxBindingValue(this.loaders)
    Object.freeze(this)
  }
  difference(other: MdxBindingReceipt): MdxBindingDifference | undefined {
    return compareMdxBindingValues(this.#snapshot, other.#snapshot)
  }
}

export function captureMdxBindingReceipt(
  input: MdxBindingInput,
): MdxBindingReceipt {
  return new MdxBindingReceipt(input)
}

export type MdxBindingOwner = {
  readonly ownerID: string
  readonly configFile: string
}
export type MdxBindingState =
  | { readonly kind: 'unbound' }
  | { readonly kind: 'bound'; readonly receipt: MdxBindingReceipt }
  | { readonly kind: 'released' }
export type MdxBinding = {
  readonly state: () => MdxBindingState
  readonly accept: (receipt: MdxBindingReceipt) => MdxBindingReceipt
  readonly release: () => void
}

export class MdxBindingError extends Error {
  readonly name = 'MdxBindingError'
  readonly path: string
  constructor(
    readonly owner: MdxBindingOwner,
    cause: MdxBindingDifference,
  ) {
    super(
      `${owner.configFile}:1:1: devup-ui cannot rebind owner ${owner.ownerID}; first difference ${cause.path}: ${cause.reason}; mixing prepared CSS/class allocation with another pre-extraction pipeline is unsafe; use the same pipeline within this owner/evaluation, e.g. a module-level plugin instead of recreating a phase-function closure for each compiler`,
      { cause },
    )
    this.path = cause.path
  }
}

/** One value owned by SessionOwner; deliberately no global registry or lifecycle wiring. */
export function createMdxBinding(owner: MdxBindingOwner): MdxBinding {
  const identity = Object.freeze({ ...owner })
  let state: MdxBindingState = Object.freeze({ kind: 'unbound' })
  return Object.freeze({
    state: () => state,
    accept(receipt: MdxBindingReceipt) {
      switch (state.kind) {
        case 'unbound':
          state = Object.freeze({ kind: 'bound', receipt })
          return receipt
        case 'bound': {
          const difference = state.receipt.difference(receipt)
          if (difference) throw new MdxBindingError(identity, difference)
          return state.receipt
        }
        case 'released':
          throw new MdxBindingError(
            identity,
            new MdxBindingDifference(
              'owner.state',
              'released owner cannot revive',
            ),
          )
        default:
          return state satisfies never
      }
    },
    release() {
      state = Object.freeze({ kind: 'released' })
    },
  })
}
