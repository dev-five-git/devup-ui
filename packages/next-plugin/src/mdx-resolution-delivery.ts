import { resolve } from 'node:path'

import { computeReachableFiles } from '@devup-ui/plugin-utils'

import {
  unionResolutionProof,
  verifyResolutionProof,
} from './mdx-resolution-proof'
import type { MdxCacheEntry } from './mdx-source-cache'
import type { MdxInputFingerprint } from './mdx-source-freshness'
import type { runMdxSourcePreparation } from './mdx-source-run'
import type { MdxExtractionReport } from './mdx-source-types'

export function deliverResolutionProof(
  run: Awaited<ReturnType<typeof runMdxSourcePreparation>>,
  root: string,
  reports: readonly MdxExtractionReport[],
) {
  const graphProof = run.resolution.snapshot()
  const byKey = new Map(
    graphProof.map((input) => [`${input.kind}:${input.path}`, input]),
  )
  for (const [filename, entry] of run.entries) {
    const reached = new Set(
      computeReachableFiles({
        srcDir: root,
        graph: run.graph,
        entries: [filename],
      }),
    )
    reached.add(filename)
    const requestInputs = (run.graph.requests ?? []).flatMap(
      (request): readonly MdxInputFingerprint[] => {
        if (!reached.has(request.importer)) return []
        switch (request.outcome.kind) {
          case 'resolved':
            return [
              ...request.outcome.inputs.fileDependencies.map(
                (path) => `file:${path}`,
              ),
              ...request.outcome.inputs.missingDependencies.map(
                (path) => `missing:${path}`,
              ),
            ].flatMap((key) => {
              const input = byKey.get(key)
              return input ? [input] : []
            })
          case 'external':
          case 'ignored':
          case 'excluded':
          case 'unresolved':
          case 'error':
            return []
        }
      },
    )
    const extraction = reports
      .filter((report) => resolve(root, report.filename) === filename)
      .flatMap((report) => report.resolutionInputs ?? [])
    const proof = unionResolutionProof(
      entry.resolutionInputs ?? [],
      requestInputs,
      extraction,
    )
    verifyResolutionProof(filename, proof)
    const next: MdxCacheEntry = Object.freeze({
      ...entry,
      resolutionInputs: proof,
    })
    run.entries.set(filename, next)
  }
  const proof = unionResolutionProof(
    graphProof,
    ...reports.map((report) => report.resolutionInputs ?? []),
  )
  verifyResolutionProof(root, proof)
  return proof
}
