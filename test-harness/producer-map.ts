import { CoverageError } from './lcov'
import { array, integer, object, text } from './producer-data'

interface Mapping {
  readonly column: number
  readonly originalLine: number
}
export interface ProducerMap {
  readonly input: string
  readonly bytes: Buffer
  readonly starts: readonly number[]
  readonly rows: readonly (readonly Mapping[])[]
}

export function decodeMap(code: string, sourceMapURL: string): ProducerMap {
  const emitted =
    /\/\/# sourceMappingURL=(data:application\/json;base64,[^\r\n]+)/.exec(
      code,
    )?.[1]
  if (!emitted || emitted !== sourceMapURL)
    throw new CoverageError('missing or contradictory executed source map')
  const map = object(
    JSON.parse(
      Buffer.from(emitted.slice(emitted.indexOf(',') + 1), 'base64').toString(
        'utf8',
      ),
    ),
  )
  if (
    integer(map['version']) !== 3 ||
    array(map['sources']).length !== 1 ||
    array(map['sourcesContent']).length !== 1
  )
    throw new CoverageError('unsupported producer source map inventory')
  const input = text(array(map['sourcesContent'])[0])
  const inputLines = input.split('\n').length
  const rows: Mapping[][] = []
  let source = 0
  let originalLine = 0
  let originalColumn = 0
  for (const line of text(map['mappings']).split(';')) {
    let column = 0
    const row: Mapping[] = []
    for (const segment of line.split(',').filter(Boolean)) {
      const values = decodeVlq(segment)
      const first = values[0]
      if (first === undefined)
        throw new CoverageError('empty source map segment')
      column += first
      if (values.length !== 4 && values.length !== 5)
        throw new CoverageError('unmapped source map segment')
      const [, sourceDelta = 0, lineDelta = 0, columnDelta = 0] = values
      source += sourceDelta
      originalLine += lineDelta
      originalColumn += columnDelta
      if (
        source !== 0 ||
        originalLine < 0 ||
        originalLine >= inputLines ||
        originalColumn < 0 ||
        column < 0
      )
        throw new CoverageError('invalid producer source map location')
      row.push({ column, originalLine })
    }
    rows.push(row)
  }
  const bytes = Buffer.from(code)
  const starts = [0]
  for (let offset = 0; offset < bytes.length; offset++)
    if (bytes[offset] === 10) starts.push(offset + 1)
  return { input, bytes, starts, rows }
}

function decodeVlq(segment: string): number[] {
  const result: number[] = []
  const alphabet =
    'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
  let value = 0
  let shift = 0
  for (const character of segment) {
    const digit = alphabet.indexOf(character)
    if (digit < 0 || shift > 48)
      throw new CoverageError('invalid source map VLQ')
    value += (digit & 31) * 2 ** shift
    if (digit & 32) shift += 5
    else {
      result.push(value % 2 ? -Math.floor(value / 2) : Math.floor(value / 2))
      value = 0
      shift = 0
    }
  }
  if (shift !== 0) throw new CoverageError('unterminated source map VLQ')
  return result
}

export function mappedLines(
  map: ProducerMap,
  range: { readonly startOffset: number; readonly endOffset: number },
): number[] {
  const result: number[] = []
  const min = Math.min(range.startOffset, range.endOffset)
  const max = Math.max(range.startOffset, range.endOffset)
  if (max > map.bytes.length)
    throw new CoverageError('producer range exceeds executed source')
  let line = 0
  for (let offset = min; offset < max; offset++) {
    while ((map.starts[line + 1] ?? Infinity) <= offset) line++
    const column = offset - (map.starts[line] ?? 0)
    if (column === 0) continue
    const point = map.rows[line]?.findLast((point) => point.column <= column)
    if (point) result.push(point.originalLine + 1)
  }
  return result
}
