import type { ResolutionInputs } from '@devup-ui/plugin-utils'

import {
  changedMdxInput,
  fingerprintMdxInput,
  MdxFreshnessError,
  type MdxInputFingerprint,
} from './mdx-source-freshness'

export function createResolutionProof(filename: string) {
  const inputs = new Map<string, MdxInputFingerprint>()
  let failure: unknown
  let active = true
  function observe(observation: ResolutionInputs) {
    if (!active) return
    try {
      for (const [kind, paths] of [
        ['file', observation.fileDependencies],
        ['missing', observation.missingDependencies],
      ] as const)
        for (const path of paths) {
          const key = `${kind}:${path}`
          if (inputs.has(key)) continue
          const input = fingerprintMdxInput(filename, {
            kind,
            path,
            loader: 'devup/resolution',
          })
          if (kind === 'missing' && input.fingerprint !== 'absent')
            throw new MdxFreshnessError(
              filename,
              input,
              'observed missing input appeared during resolution',
            )
          inputs.set(key, input)
        }
    } catch (cause) {
      failure ??= cause
    }
  }
  return Object.freeze({
    observe,
    snapshot() {
      if (failure !== undefined) throw failure
      return Object.freeze(
        [...inputs.values()].sort((a, b) => {
          const first = `${a.kind}:${a.path}`
          const second = `${b.kind}:${b.path}`
          return Number(first > second) - Number(first < second)
        }),
      )
    },
    retire() {
      active = false
    },
  })
}

export function verifyResolutionProof(
  filename: string,
  inputs: readonly MdxInputFingerprint[],
) {
  const changed = changedMdxInput(filename, inputs)
  if (changed)
    throw new MdxFreshnessError(
      filename,
      changed,
      'resolution input changed before publication',
    )
}

export function unionResolutionProof(
  ...proofs: readonly (readonly MdxInputFingerprint[])[]
) {
  const inputs = new Map<string, MdxInputFingerprint>()
  for (const proof of proofs)
    for (const input of proof)
      if (!inputs.has(`${input.kind}:${input.path}`))
        inputs.set(`${input.kind}:${input.path}`, Object.freeze({ ...input }))
  return Object.freeze([...inputs.values()])
}
