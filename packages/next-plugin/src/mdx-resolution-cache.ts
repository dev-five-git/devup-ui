import { isMdxRecord } from './mdx-pipeline'
import type { MdxDependencyReport } from './mdx-prepare-dependencies'
import type { MdxInputFingerprint } from './mdx-source-freshness'

function dependencyReport(value: unknown): MdxDependencyReport {
  if (
    !isMdxRecord(value) ||
    (value.kind !== 'file' &&
      value.kind !== 'build' &&
      value.kind !== 'missing' &&
      value.kind !== 'context') ||
    typeof value.path !== 'string' ||
    typeof value.loader !== 'string'
  )
    throw new TypeError('Invalid MDX restart dependency report')
  return Object.freeze({
    kind: value.kind,
    path: value.path,
    loader: value.loader,
  })
}

export function readResolutionRestartProof(
  value: unknown,
): readonly MdxInputFingerprint[] {
  if (!Array.isArray(value))
    throw new TypeError('Invalid MDX restart resolution proof')
  return Object.freeze(
    value.map((input: unknown) => {
      const report = dependencyReport(input)
      if (
        !isMdxRecord(input) ||
        (report.kind !== 'file' && report.kind !== 'missing') ||
        typeof input.fingerprint !== 'string' ||
        typeof input.mtime !== 'number' ||
        (report.kind === 'missing' && input.fingerprint !== 'absent')
      )
        throw new TypeError('Invalid MDX restart resolution proof')
      return Object.freeze({
        ...report,
        fingerprint: input.fingerprint,
        mtime: input.mtime,
      })
    }),
  )
}

export function readCompilerRestartReports(
  value: unknown,
): readonly MdxDependencyReport[] {
  if (value === undefined) return Object.freeze([])
  if (!Array.isArray(value))
    throw new TypeError('Invalid MDX restart compiler reports')
  return Object.freeze(value.map(dependencyReport))
}
