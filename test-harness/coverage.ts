import { CoverageError, type SourceCoverage } from './lcov'
import {
  isVerifiedSourceCoverage,
  type VerifiedSourceCoverage,
} from './producer-coverage'

export function mergeCoverage(
  reports: readonly (readonly SourceCoverage[])[],
): SourceCoverage[] {
  const sources = new Map<string, VerifiedSourceCoverage>()
  for (const report of reports) {
    const seen = new Set<string>()
    for (const source of report) {
      if (!isVerifiedSourceCoverage(source))
        throw new CoverageError('unproven coverage universe')
      if (seen.has(source.file))
        throw new CoverageError(`duplicate source ${source.file}`)
      seen.add(source.file)
      const previous = sources.get(source.file)
      if (!previous) {
        sources.set(source.file, source)
        continue
      }
      sources.set(source.file, previous.merge(source))
    }
  }
  return [...sources.values()].sort((a, b) => a.file.localeCompare(b.file))
}

export function auditCoverage(
  sources: readonly SourceCoverage[],
  required: readonly string[],
): void {
  if (sources.length === 0) throw new CoverageError('empty merged report')
  for (const source of sources) {
    if (!isVerifiedSourceCoverage(source))
      throw new CoverageError('unproven coverage universe')
  }
  for (const file of required) {
    if (!sources.some((source) => source.file === file))
      throw new CoverageError(`missing source ${file}`)
  }
  for (const source of sources) {
    if (
      source.hitFunctions !== source.foundFunctions ||
      [...source.lines.values()].some((hits) => hits === 0)
    ) {
      throw new CoverageError(`below 100% lines/functions: ${source.file}`)
    }
  }
}

export function writeLcov(sources: readonly SourceCoverage[]): string {
  return sources
    .map((source) =>
      [
        'TN:',
        `SF:${source.file}`,
        ...[...source.functions].flatMap(([name, fn]) => [
          `FN:${fn.line},${name}`,
          `FNDA:${fn.hits},${name}`,
        ]),
        `FNF:${source.foundFunctions}`,
        `FNH:${source.hitFunctions}`,
        ...[...source.lines]
          .sort(([a], [b]) => a - b)
          .map(([line, hits]) => `DA:${line},${hits}`),
        `LF:${source.lines.size}`,
        `LH:${[...source.lines.values()].filter((hits) => hits > 0).length}`,
        ...[...source.branches].map(
          ([branch, hits]) => `BRDA:${branch},${hits}`,
        ),
        `BRF:${source.branches.size}`,
        `BRH:${[...source.branches.values()].filter((hits) => hits > 0).length}`,
        'end_of_record',
        '',
      ].join('\n'),
    )
    .join('')
}

export function coverageTable(sources: readonly SourceCoverage[]): string {
  const foundFunctions = sources.reduce(
    (sum, source) => sum + source.foundFunctions,
    0,
  )
  const hitFunctions = sources.reduce(
    (sum, source) => sum + source.hitFunctions,
    0,
  )
  const foundLines = sources.reduce((sum, source) => sum + source.lines.size, 0)
  const hitLines = sources.reduce(
    (sum, source) =>
      sum + [...source.lines.values()].filter((hits) => hits > 0).length,
    0,
  )
  return [
    'File | % Funcs | % Lines | Uncovered Line #s',
    `All files | ${percentage(hitFunctions, foundFunctions)} | ${percentage(hitLines, foundLines)} |`,
    ...sources.map((source) => {
      const uncovered = [...source.lines]
        .filter(([, hits]) => hits === 0)
        .map(([line]) => line)
      const functions =
        source.foundFunctions === 0
          ? 100
          : (source.hitFunctions / source.foundFunctions) * 100
      const lines =
        source.lines.size === 0
          ? 100
          : ((source.lines.size - uncovered.length) / source.lines.size) * 100
      return `${source.file} | ${functions.toFixed(2)} | ${lines.toFixed(2)} | ${uncovered.join(',')}`
    }),
  ].join('\n')
}

function percentage(hits: number, total: number): string {
  return (total === 0 ? 100 : (hits / total) * 100).toFixed(2)
}
