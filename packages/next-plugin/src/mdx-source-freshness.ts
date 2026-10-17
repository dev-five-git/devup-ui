import { createHash } from 'node:crypto'
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { isAbsolute } from 'node:path'

import type { MdxDependencyReport } from './mdx-prepare-dependencies'
import type { PreparedMdx } from './mdx-prepare-result'

export type MdxInputFingerprint = MdxDependencyReport & {
  readonly fingerprint: string
  readonly mtime: number
}

export class MdxFreshnessError extends Error {
  readonly name = 'MdxFreshnessError'
  constructor(
    readonly filename: string,
    readonly input: MdxDependencyReport,
    readonly detail: string,
  ) {
    super(
      `${filename}:1:1: MDX preparation cannot use input \`${input.path}\` reported by ${input.loader}: ${detail}; needs re-preparation before CSS publication`,
    )
  }
}

export function sourceHash(source: string | Buffer): string {
  return createHash('sha256').update(source).digest('hex')
}

export function fingerprintMdxInput(
  filename: string,
  input: MdxDependencyReport,
): MdxInputFingerprint {
  if (!isAbsolute(input.path))
    throw new MdxFreshnessError(
      filename,
      input,
      'dependency path is not absolute',
    )
  try {
    const stat = statSync(input.path, { throwIfNoEntry: false })
    switch (input.kind) {
      case 'missing':
        return Object.freeze({
          ...input,
          fingerprint: stat ? 'present' : 'absent',
          mtime: 0,
        })
      case 'file':
      case 'build':
        if (!stat?.isFile())
          throw new MdxFreshnessError(
            filename,
            input,
            'reported file is not readable',
          )
        return Object.freeze({
          ...input,
          fingerprint: sourceHash(readFileSync(input.path)),
          mtime: Number(stat.mtime),
        })
      case 'context': {
        if (!stat?.isDirectory())
          throw new MdxFreshnessError(
            filename,
            input,
            'reported directory is not readable',
          )
        const entries = readdirSync(input.path, { withFileTypes: true })
          .map(
            (entry) =>
              [
                entry.name,
                entry.isFile()
                  ? 'file'
                  : entry.isDirectory()
                    ? 'directory'
                    : entry.isSymbolicLink()
                      ? 'symlink'
                      : 'other',
              ] as const,
          )
          .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
        return Object.freeze({
          ...input,
          fingerprint: sourceHash(JSON.stringify(entries)),
          mtime: Number(stat.mtime),
        })
      }
      default:
        return input.kind satisfies never
    }
  } catch (cause) {
    if (cause instanceof MdxFreshnessError) throw cause
    throw new MdxFreshnessError(
      filename,
      input,
      cause instanceof Error ? cause.message : String(cause),
    )
  }
}

export function reportedMdxInputs(
  output: PreparedMdx,
): readonly MdxDependencyReport[] {
  const inputs = new Map<string, MdxDependencyReport>()
  for (const [kind, paths] of [
    ['file', output.dependencies],
    ['context', output.contextDependencies],
    ['missing', output.missingDependencies],
    ['build', output.buildDependencies],
  ] as const) {
    for (const path of paths)
      inputs.set(
        `${kind}:${path}`,
        Object.freeze({
          kind,
          path,
          loader:
            output.dependencyReports.find(
              (report) => report.kind === kind && report.path === path,
            )?.loader ?? 'loader-runner/resource',
        }),
      )
  }
  return Object.freeze([...inputs.values()])
}

export function createMdxTimestampAccuracy() {
  let accuracy = 2000
  return {
    apply(mtime: number) {
      if (accuracy > 1 && mtime % 2 !== 0) accuracy = 1
      else if (accuracy > 10 && mtime % 20 !== 0) accuracy = 10
      else if (accuracy > 100 && mtime % 200 !== 0) accuracy = 100
      else if (accuracy > 1000 && mtime % 2000 !== 0) accuracy = 1000
      return accuracy
    },
  }
}

export function captureMdxFreshness(
  output: PreparedMdx,
  control: {
    readonly startedAt: number
    readonly development: boolean
    readonly accuracy: ReturnType<typeof createMdxTimestampAccuracy>
  },
): readonly MdxInputFingerprint[] {
  const inputs = reportedMdxInputs(output).map((input) =>
    fingerprintMdxInput(output.filename, input),
  )
  for (const input of inputs) {
    if (input.mtime) control.accuracy.apply(input.mtime)
    if (input.kind === 'missing' && input.fingerprint !== 'absent')
      throw new MdxFreshnessError(
        output.filename,
        input,
        'reported missing dependency appeared during compilation',
      )
  }
  if (control.development) {
    const margin = control.accuracy.apply(0)
    const changed = inputs.find(
      (input) =>
        input.kind !== 'missing' && input.mtime >= control.startedAt - margin,
    )
    if (changed)
      throw new MdxFreshnessError(
        output.filename,
        changed,
        'input timestamp overlaps the compile interval',
      )
  }
  return Object.freeze(inputs)
}

export function changedMdxInput(
  filename: string,
  inputs: readonly MdxInputFingerprint[],
): MdxInputFingerprint | undefined {
  return inputs.find((input) => {
    try {
      return (
        fingerprintMdxInput(filename, input).fingerprint !== input.fingerprint
      )
    } catch (cause) {
      if (cause instanceof MdxFreshnessError) return true
      throw cause
    }
  })
}
