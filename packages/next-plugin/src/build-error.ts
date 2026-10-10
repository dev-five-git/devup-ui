const LOCATED_MESSAGE = /^.+:\d+:\d+: /

interface LocatedErrorFields {
  /** Where the problem is: a source file or the directory being planned */
  file: string
  /** What was being done, as a noun phrase: `devup-ui prewarm` */
  what: string
  /** The construct that cannot be used: a specifier, `JSON`, `buildStaticImportGraph` */
  code: string
  /** What the build needs for this to work */
  needs: string
  cause: unknown
}

function describe(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause)
}

/**
 * An error in the project's `file:line:col: <what> cannot use `<code>` at build
 * time: <what it needs>` form. A cause that already says where it is (an
 * extraction error, for instance) is kept as it is, so a location is never
 * written twice.
 */
export function locatedError({
  file,
  what,
  code,
  needs,
  cause,
}: LocatedErrorFields): Error {
  if (cause instanceof Error && LOCATED_MESSAGE.test(cause.message)) {
    return cause
  }
  return new Error(
    `${file}:1:1: ${what} cannot use \`${code}\` at build time: ${describe(cause)}; needs ${needs}`,
    { cause },
  )
}
