/** A line break as the engine counts lines: CRLF, LF, CR, U+2028 and U+2029 */
const LINE_BREAK = new RegExp(
  `\r\n|[\n\r${String.fromCharCode(0x2028, 0x2029)}]`,
)

/** Lines of source shown around the reported line */
const CONTEXT_LINES = 2

/**
 * The error Metro shows for a build error of the engine. The engine leads
 * each error with `filename:line:column`; Metro prints an error with `loc` as
 * a transform error at that place, with `codeFrame` under it, the way it
 * prints a Babel syntax error.
 */
export class DevupUIBuildError extends SyntaxError {
  static {
    // Metro's workers send an error by its constructor name, which the
    // minified build would lose
    Object.defineProperty(this, 'name', { value: 'DevupUIBuildError' })
  }
  readonly name = 'DevupUIBuildError'
  /** Babel's location shape: a 1-based line and a 0-based column */
  readonly loc: { readonly line: number; readonly column: number }
  readonly codeFrame: string

  constructor(message: string, line: number, column: number, source: string) {
    super(message)
    this.loc = { line, column: column - 1 }
    this.codeFrame = codeFrame(source, line, column)
  }
}

/**
 * The lines of `source` around `line`, numbered, with a caret under `column`
 * (both 1-based)
 */
export function codeFrame(source: string, line: number, column: number) {
  const lines = source.split(LINE_BREAK)
  const first = Math.max(line - CONTEXT_LINES, 1)
  const last = Math.min(line + CONTEXT_LINES, lines.length)
  const width = String(last).length
  const frame: string[] = []
  for (let number = first; number <= last; number++) {
    const gutter = String(number).padStart(width)
    const text = lines[number - 1]
    frame.push(
      `${number === line ? '>' : ' '} ${gutter} |${text ? ` ${text}` : ''}`,
    )
    if (number === line) {
      frame.push(`  ${' '.repeat(width)} | ${' '.repeat(column - 1)}^`)
    }
  }
  return frame.join('\n')
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * `error` of the engine compiling `filename` as Metro reports it: located at
 * its first `filename:line:column` when it has one, as it is otherwise
 */
export function locatedError(
  error: unknown,
  filename: string,
  source: string,
): unknown {
  if (!(error instanceof Error)) return error
  const location = new RegExp(
    `^${escapeRegExp(filename)}:(\\d+):(\\d+): `,
    'm',
  ).exec(error.message)
  if (!location) return error
  const located = new DevupUIBuildError(
    error.message,
    Number(location[1]),
    Number(location[2]),
    source,
  )
  located.cause = error
  return located
}
