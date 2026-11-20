const BASE64 =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'

/** A place in code: a 1-based line and a 0-based column, as in Babel */
export interface Position {
  line: number
  column: number
}

/** A mapping from a place in the compiled code to one in the source */
interface Segment extends Position {
  readonly source: Position
}

/** The numbers of one Base64 VLQ segment */
function decodeSegment(text: string): number[] {
  const values: number[] = []
  let value = 0
  let shift = 0
  for (const character of text) {
    const digit = BASE64.indexOf(character)
    value += (digit & 31) << shift
    if (digit & 32) {
      shift += 5
    } else {
      values.push(value & 1 ? -(value >>> 1) : value >>> 1)
      value = 0
      shift = 0
    }
  }
  return values
}

/** The segments of a single-source map's `mappings` that name a source place */
export function decodeMappings(mappings: string): Segment[] {
  const segments: Segment[] = []
  let sourceLine = 0
  let sourceColumn = 0
  mappings.split(';').forEach((text, index) => {
    let column = 0
    for (const group of text.split(',')) {
      const values = decodeSegment(group)
      if (values.length === 0) continue
      column += values[0]
      if (values.length < 4) continue
      sourceLine += values[2]
      sourceColumn += values[3]
      segments.push({
        line: index + 1,
        column,
        source: { line: sourceLine + 1, column: sourceColumn },
      })
    }
  })
  return segments
}

/**
 * The source place of a place in the compiled code: the last segment at or
 * before it, moved by the columns between them on the same line, or that
 * segment's place for code the compiler inserted on lines of its own.
 * `undefined` before the first segment.
 */
export function createLocator(segments: readonly Segment[]) {
  return (place: Position): Position | undefined => {
    let low = 0
    let high = segments.length - 1
    let found: Segment | undefined
    while (low <= high) {
      const middle = (low + high) >> 1
      const segment = segments[middle]
      if (
        segment.line < place.line ||
        (segment.line === place.line && segment.column <= place.column)
      ) {
        found = segment
        low = middle + 1
      } else {
        high = middle - 1
      }
    }
    if (!found) return undefined
    return found.line === place.line
      ? {
          line: found.source.line,
          column: found.source.column + place.column - found.column,
        }
      : { ...found.source }
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isPosition(value: unknown): value is Position {
  return (
    isObject(value) &&
    typeof value.line === 'number' &&
    typeof value.column === 'number'
  )
}

/**
 * Move every `loc` in the Babel AST `root`, built from the compiled code, to
 * the source: Metro writes the source map of a module from these locations
 * and pairs it with the file as it is on disk.
 */
export function remapLocations(root: unknown, map: string): void {
  const { mappings }: { mappings: string } = JSON.parse(map)
  const locate = createLocator(decodeMappings(mappings))
  const seen = new WeakSet<object>()
  const move = (position: unknown) => {
    if (!isPosition(position) || seen.has(position)) return
    seen.add(position)
    const source = locate(position)
    if (!source) return
    position.line = source.line
    position.column = source.column
  }
  const visit = (node: unknown) => {
    if (!isObject(node) || seen.has(node)) return
    seen.add(node)
    for (const [key, value] of Object.entries(node)) {
      if (key === 'loc' && isObject(value)) {
        move(value.start)
        move(value.end)
      } else {
        visit(value)
      }
    }
  }
  visit(root)
}
