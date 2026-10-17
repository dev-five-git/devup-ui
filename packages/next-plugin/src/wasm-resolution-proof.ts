import type { ResolutionInputObserver } from '@devup-ui/plugin-utils'

import {
  createResolutionProof,
  unionResolutionProof,
  verifyResolutionProof,
} from './mdx-resolution-proof'
import type { MdxInputFingerprint } from './mdx-source-freshness'
import type { DevupWasm } from './wasm'

type Collector = ReturnType<typeof createResolutionProof>
const engines = new WeakMap<
  DevupWasm,
  {
    readonly setup: readonly MdxInputFingerprint[]
    active?: Collector
  }
>()
const outputs = new WeakMap<object, readonly MdxInputFingerprint[]>()

export function createEngineResolutionProof(
  wasm: DevupWasm,
  root: string,
  observer?: ResolutionInputObserver,
) {
  const setup = createResolutionProof(root)
  let constructing = true
  return {
    observe: ((inputs) => {
      if (constructing) setup.observe(inputs)
      else engines.get(wasm)?.active?.observe(inputs)
      if (constructing || engines.get(wasm)?.active) observer?.(inputs)
    }) satisfies ResolutionInputObserver,
    install() {
      constructing = false
      engines.set(wasm, { setup: setup.snapshot() })
      setup.retire()
    },
  }
}

export function withExtractionResolutionProof<T extends object>(
  wasm: DevupWasm,
  filename: string,
  extract: () => T,
): T {
  const state = engines.get(wasm)
  if (!state) return extract()
  const active = createResolutionProof(filename)
  state.active = active
  try {
    verifyResolutionProof(filename, state.setup)
    const output = extract()
    try {
      const proof = unionResolutionProof(state.setup, active.snapshot())
      verifyResolutionProof(filename, proof)
      outputs.set(output, proof)
      return output
    } catch (cause) {
      if ('free' in output && typeof output.free === 'function') output.free()
      throw cause
    }
  } finally {
    active.retire()
    delete state.active
  }
}

export function extractionResolutionProof(
  output: object,
): readonly MdxInputFingerprint[] {
  return outputs.get(output) ?? Object.freeze([])
}
