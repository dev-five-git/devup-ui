import { readFileSync } from 'node:fs'
import { relative, resolve } from 'node:path'

import {
  fingerprintMdxInput,
  type MdxInputFingerprint,
  sourceHash,
} from './mdx-source-freshness'
import type {
  MdxBuildBinding,
  MdxExtractionReport,
  MdxNativeExpectation,
} from './mdx-source-types'
import { EXTRACTABLE_EXTENSION } from './prewarm'
import type { CoordinatorInput } from './state'

export function sourceInputKey(root: string, filename: string): string {
  return relative(root, resolve(root, filename)).replaceAll('\\', '/')
}

export function collectMdxOrdinaryInputs(
  binding: MdxBuildBinding,
  files: readonly string[],
) {
  const ordinaryInputs: CoordinatorInput[] = []
  const pending: MdxNativeExpectation[] = []
  const fingerprints: MdxInputFingerprint[] = []
  for (const file of [
    ...new Set(
      files.map((file) => resolve(binding.effectiveAppContext.root, file)),
    ),
  ].sort()) {
    const filename = resolve(binding.effectiveAppContext.root, file)
    if (!EXTRACTABLE_EXTENSION.test(filename)) {
      if (!/\.mdx?$/i.test(filename))
        fingerprints.push(
          fingerprintMdxInput(filename, {
            kind: 'file',
            path: filename,
            loader: 'devup/extractor',
          }),
        )
      continue
    }
    const eligibility = binding.ordinaryEligibility(filename)
    switch (eligibility.kind) {
      case 'native-required':
        pending.push(Object.freeze({ ...eligibility.expectation, filename }))
        break
      case 'disk-first':
        {
          const source = readFileSync(filename, 'utf8')
          ordinaryInputs.push(
            Object.freeze({
              filename: sourceInputKey(
                binding.effectiveAppContext.root,
                filename,
              ),
              resourcePath: filename,
              source,
              dependencies: Object.freeze([]),
              stamps: Object.freeze({}),
              backing: sourceHash(source),
            }),
          )
          const input = fingerprintMdxInput(filename, {
            kind: 'file',
            path: filename,
            loader: 'native/disk-first',
          })
          fingerprints.push(
            Object.freeze({ ...input, fingerprint: sourceHash(source) }),
          )
        }
        break
      default:
        eligibility satisfies never
    }
  }
  return {
    ordinaryInputs: Object.freeze(ordinaryInputs),
    pending: Object.freeze(pending),
    fingerprints: Object.freeze(fingerprints),
  }
}

export function ordinaryExtractionEvidence(
  inputs: readonly CoordinatorInput[],
  proof: {
    readonly root: string
    readonly reports: readonly MdxExtractionReport[]
    readonly fingerprints: readonly MdxInputFingerprint[]
  },
): readonly CoordinatorInput[] {
  return Object.freeze(
    inputs.map((input) => {
      const dependencies =
        proof.reports.find(
          (report) =>
            sourceInputKey(proof.root, report.filename) === input.filename,
        )?.dependencies ?? []
      const paths = new Set(
        dependencies.map((path) => resolve(proof.root, path)),
      )
      return Object.freeze({
        ...input,
        dependencies: Object.freeze(
          dependencies.map((path) => sourceInputKey(proof.root, path)),
        ),
        stamps: Object.freeze(
          Object.fromEntries(
            proof.fingerprints
              .filter((stamp) => paths.has(stamp.path))
              .map((stamp) => [stamp.path, stamp.fingerprint]),
          ),
        ),
      })
    }),
  )
}

export function overlayMdxOrdinaryInputs(
  previous: readonly CoordinatorInput[],
  additions: readonly CoordinatorInput[],
): readonly CoordinatorInput[] {
  return Object.freeze(
    [
      ...new Map(
        [...previous, ...additions].map((input) => [
          input.filename,
          Object.freeze({
            ...input,
            dependencies: Object.freeze([...input.dependencies]),
            stamps: Object.freeze({ ...input.stamps }),
          }),
        ]),
      ).values(),
    ].sort((a, b) => a.filename.localeCompare(b.filename)),
  )
}
