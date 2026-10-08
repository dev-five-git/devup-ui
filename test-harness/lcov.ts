export class CoverageError extends Error {
  constructor(readonly detail: string) {
    super(`Coverage: ${detail}`)
    this.name = 'CoverageError'
  }
}

/** Mutable accumulator; all counts refer to the same source bytes. */
export interface SourceCoverage {
  readonly file: string
  readonly lines: Map<number, number>
  readonly functions: Map<string, { readonly line: number; hits: number }>
  readonly branches: Map<string, number>
  foundFunctions: number
  hitFunctions: number
}

function count(value: string): number {
  const result = Number(value)
  if (!/^\d+$/.test(value) || !Number.isSafeInteger(result)) {
    throw new CoverageError(`invalid count ${value}`)
  }
  return result
}

export function parseLcov(text: string): SourceCoverage[] {
  const result: SourceCoverage[] = []
  let source: SourceCoverage | undefined
  const summaries = new Map<string, number>()
  for (const row of text.split(/\r?\n/)) {
    if (!row || row.startsWith('TN:')) continue
    if (row.startsWith('SF:')) {
      if (source) throw new CoverageError('unterminated source record')
      source = {
        file: row.slice(3).replaceAll('\\', '/'),
        lines: new Map(),
        functions: new Map(),
        branches: new Map(),
        foundFunctions: 0,
        hitFunctions: 0,
      }
      summaries.clear()
      continue
    }
    if (!source) throw new CoverageError(`record outside source: ${row}`)
    const separator = row.indexOf(':')
    const tag = row.slice(0, separator)
    const value = row.slice(separator + 1)
    switch (tag) {
      case 'DA': {
        const [line = '', hits = '', checksum] = value.split(',')
        if (checksum) throw new CoverageError('checksummed DA is unsupported')
        const identity = count(line)
        if (source.lines.has(identity)) throw new CoverageError('duplicate DA')
        source.lines.set(identity, count(hits))
        break
      }
      case 'FN': {
        const [line = '', ...name] = value.split(',')
        const identity = name.join(',')
        if (!identity || source.functions.has(identity)) {
          throw new CoverageError('ambiguous function name')
        }
        source.functions.set(identity, { line: count(line), hits: 0 })
        break
      }
      case 'FNDA': {
        const comma = value.indexOf(',')
        const fn = source.functions.get(value.slice(comma + 1))
        if (!fn) throw new CoverageError('FNDA has no function definition')
        fn.hits += count(value.slice(0, comma))
        break
      }
      case 'BRDA': {
        const [line = '', block = '', branch = '', hits = ''] = value.split(',')
        const identity = `${count(line)},${count(block)},${count(branch)}`
        if (source.branches.has(identity))
          throw new CoverageError('duplicate branch')
        source.branches.set(identity, hits === '-' ? 0 : count(hits))
        break
      }
      case 'FNF':
      case 'FNH':
      case 'LF':
      case 'LH':
      case 'BRF':
      case 'BRH':
        if (summaries.has(tag)) throw new CoverageError(`duplicate ${tag}`)
        summaries.set(tag, count(value))
        break
      default:
        if (row !== 'end_of_record')
          throw new CoverageError(`unknown record ${row}`)
        source.foundFunctions = summaries.get('FNF') ?? 0
        source.hitFunctions = summaries.get('FNH') ?? 0
        if (
          !summaries.has('FNF') ||
          !summaries.has('FNH') ||
          source.hitFunctions > source.foundFunctions ||
          summaries.get('LF') !== source.lines.size ||
          summaries.get('LH') !==
            [...source.lines.values()].filter((hits) => hits > 0).length ||
          (source.functions.size > 0 &&
            (source.functions.size !== source.foundFunctions ||
              [...source.functions.values()].filter((fn) => fn.hits > 0)
                .length !== source.hitFunctions)) ||
          (source.branches.size > 0 &&
            (summaries.get('BRF') !== source.branches.size ||
              summaries.get('BRH') !==
                [...source.branches.values()].filter((hits) => hits > 0)
                  .length))
        )
          throw new CoverageError(`inconsistent summaries for ${source.file}`)
        result.push(source)
        source = undefined
    }
  }
  if (source || result.length === 0)
    throw new CoverageError('incomplete LCOV report')
  return result
}
