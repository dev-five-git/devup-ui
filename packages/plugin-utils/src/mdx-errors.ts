import { posix, win32 } from 'node:path'

type SourceMap = {
  readonly mappings: string
  readonly sources: readonly string[]
  readonly names: readonly string[]
  readonly sourceRoot: string
}
type Segment = {
  readonly generated: number
  readonly original?: {
    readonly source: string
    readonly line: number
    readonly column: number
  }
}
const BASE64 =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'

class MdxSourceMapError extends Error {
  constructor(filename: string, cause: unknown) {
    super(
      `${filename}:1:1: Invalid MDX source map: ${cause instanceof Error ? cause.message : String(cause)}`,
      { cause },
    )
    this.name = 'MdxSourceMapError'
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function strings(value: unknown): value is string[] {
  return (
    Array.isArray(value) && value.every((entry) => typeof entry === 'string')
  )
}

function parseMap(input: unknown): SourceMap {
  const value: unknown = typeof input === 'string' ? JSON.parse(input) : input
  if (!isRecord(value)) throw new TypeError('Expected a source-map object')
  if (
    value.version !== 3 ||
    typeof value.mappings !== 'string' ||
    !strings(value.sources) ||
    (value.names !== undefined && !strings(value.names)) ||
    (value.sourceRoot !== undefined && typeof value.sourceRoot !== 'string')
  ) {
    throw new TypeError(
      'Expected source-map v3 mappings, sources, names and sourceRoot',
    )
  }
  return {
    mappings: value.mappings,
    sources: value.sources,
    names: value.names ?? [],
    sourceRoot: value.sourceRoot ?? '',
  }
}

function decode(segment: string): number[] {
  const values: number[] = []
  let value = 0
  let multiplier = 1
  for (const char of segment) {
    const digit = BASE64.indexOf(char)
    if (digit < 0) throw new TypeError('Invalid base64 VLQ digit')
    value += (digit % 32) * multiplier
    if (!Number.isSafeInteger(value) || !Number.isSafeInteger(multiplier))
      throw new RangeError('VLQ exceeds safe integer bounds')
    if (digit >= 32) {
      multiplier *= 32
    } else {
      values.push((value % 2 === 1 ? -1 : 1) * Math.floor(value / 2))
      value = 0
      multiplier = 1
    }
  }
  if (multiplier !== 1) throw new TypeError('Unterminated base64 VLQ')
  if (![1, 4, 5].includes(values.length))
    throw new TypeError('Expected 1, 4 or 5 VLQ fields')
  return values
}

function sourcePath(root: string, source: string): string {
  if (
    !root ||
    posix.isAbsolute(source) ||
    win32.isAbsolute(source) ||
    /^[a-z][\w+.-]*:/i.test(source)
  )
    return source
  if (/^[a-z][\w+.-]*:\/\//i.test(root))
    return new URL(source, `${root.replace(/\/$/, '')}/`).href
  return root.includes('\\')
    ? win32.join(root, source)
    : posix.join(root, source)
}

function decodeMap(map: SourceMap): Segment[][] {
  let source = 0
  let line = 0
  let column = 0
  let name = 0
  return map.mappings.split(';').map((encodedLine) => {
    let generated = 0
    if (!encodedLine) return []
    return encodedLine.split(',').map((encoded) => {
      const fields = decode(encoded)
      const [delta, sourceDelta, lineDelta, columnDelta, nameDelta] = fields
      const previous = generated
      generated += delta
      if (!Number.isSafeInteger(generated) || generated < previous)
        throw new RangeError(
          'Generated columns must be nonnegative and ordered',
        )
      if (fields.length === 1) return { generated }
      source += sourceDelta
      line += lineDelta
      column += columnDelta
      if (
        ![source, line, column].every(
          (value) => Number.isSafeInteger(value) && value >= 0,
        ) ||
        source >= map.sources.length
      ) {
        throw new RangeError('Original source position is out of bounds')
      }
      if (nameDelta !== undefined) {
        name += nameDelta
        if (!Number.isSafeInteger(name) || name < 0 || name >= map.names.length)
          throw new RangeError('Name index is out of bounds')
      }
      return {
        generated,
        original: {
          source: sourcePath(map.sourceRoot, map.sources[source]),
          line: line + 1,
          column: column + 1,
        },
      }
    })
  })
}

function decodeInput(input: unknown): Segment[][] {
  const value: unknown = typeof input === 'string' ? JSON.parse(input) : input
  if (!isRecord(value) || !('sections' in value))
    return decodeMap(parseMap(value))
  if (value.version !== 3 || !Array.isArray(value.sections))
    throw new TypeError('Expected source-map v3 sections')
  const rows: Segment[][] = []
  let previousLine = -1
  let previousColumn = -1
  for (const section of value.sections) {
    if (!isRecord(section) || !isRecord(section.offset))
      throw new TypeError('Expected a section offset and map')
    const { line, column } = section.offset
    if (
      typeof line !== 'number' ||
      typeof column !== 'number' ||
      !Number.isSafeInteger(line) ||
      !Number.isSafeInteger(column) ||
      line < 0 ||
      column < 0 ||
      line < previousLine ||
      (line === previousLine && column <= previousColumn)
    ) {
      throw new RangeError('Section offsets must be nonnegative and ordered')
    }
    previousLine = line
    previousColumn = column
    rows.length = line + 1
    rows[line] = (rows[line] ?? []).filter(
      (segment) => segment.generated < column,
    )
    rows[line].push({ generated: column })
    decodeInput(section.map).forEach((segments, index) => {
      const targetLine = line + index
      const shifted = segments.map((segment) => ({
        ...segment,
        generated: segment.generated + (index === 0 ? column : 0),
      }))
      if (shifted.some((segment) => !Number.isSafeInteger(segment.generated)))
        throw new RangeError('Section generated column is out of bounds')
      if (index === 0) rows[targetLine].push(...shifted)
      else rows[targetLine] = shifted
    })
  }
  return rows
}

/** Remap one-based extractor locations using zero-based source-map columns. */
export function remapMdxError(
  error: unknown,
  filename: string,
  inputSourceMap?: unknown,
): Error {
  let lines: Segment[][] = []
  if (inputSourceMap !== undefined && inputSourceMap !== null) {
    try {
      lines = decodeInput(inputSourceMap)
    } catch (cause) {
      throw new MdxSourceMapError(filename, cause)
    }
  }
  const message = error instanceof Error ? error.message : String(error)
  const remapped = message
    .split('\n')
    .map((text) => {
      if (!text.startsWith(`${filename}:`)) return text
      const location = /^(\d+):(\d+)(?=:|\s|$)/.exec(
        text.slice(filename.length + 1),
      )
      if (!location) return text
      const line = Number(location[1])
      const column = Number(location[2])
      if (
        !Number.isSafeInteger(line) ||
        !Number.isSafeInteger(column) ||
        line < 1 ||
        column < 1
      ) {
        throw new MdxSourceMapError(
          filename,
          new RangeError('Error location is out of bounds'),
        )
      }
      let selected: Segment | undefined
      for (const segment of lines[line - 1] ?? []) {
        if (segment.generated > column - 1) break
        selected = segment
      }
      const position = selected?.original
      const prefix = position
        ? `${position.source}:${position.line}:${position.column}`
        : `${filename}:${location[0]} (in compiled MDX)`
      return prefix + text.slice(filename.length + 1 + location[0].length)
    })
    .join('\n')
  return new Error(remapped, { cause: error })
}
