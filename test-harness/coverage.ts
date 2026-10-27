import { CoverageError, type SourceCoverage } from './lcov'

export function auditCoverage(
  sources: readonly SourceCoverage[],
  required: readonly string[],
): void {
  if (sources.length === 0) throw new CoverageError('empty native report')
  const seen = new Set<string>()
  for (const source of sources) {
    if (!source.file || seen.has(source.file))
      throw new CoverageError(`duplicate or empty source ${source.file}`)
    seen.add(source.file)
  }
  for (const file of required) {
    if (!seen.has(file)) throw new CoverageError(`missing source ${file}`)
  }
  const failures = sources
    .filter(
      (source) =>
        source.hitFunctions !== source.foundFunctions ||
        [...source.lines.values()].some((hits) => hits === 0),
    )
    .map((source) => ({
      file: source.file,
      zeroHitLines: [...source.lines]
        .filter(([, hits]) => hits === 0)
        .map(([line]) => line)
        .sort((a, b) => a - b),
      FNH: source.hitFunctions,
      FNF: source.foundFunctions,
      LH: [...source.lines.values()].filter((hits) => hits > 0).length,
      LF: source.lines.size,
    }))
  if (failures.length > 0)
    throw new CoverageError(
      `below 100% lines/functions: ${JSON.stringify(failures)}`,
    )
}
