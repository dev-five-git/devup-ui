// Adapted from Microsoft TypeScript 6.0.3 (Apache-2.0), lib/typescript.js:4805-5073.
// Resolution pins the actual installed JS authority, not native TypeScript 7.0.2.
const compiler = [6, 0, 3] as const
type Version = readonly [number, number, number, readonly string[]]
type PartialVersion = {
  readonly version: Version
  readonly major: string
  readonly minor: string
  readonly patch: string
}
const wildcard = (part: string) => part === '*' || part === 'x' || part === 'X'
export class TypeScriptVersionRangeError extends Error {
  constructor(
    readonly range: string,
    readonly component: 'prerelease' | 'build',
  ) {
    super(
      `Invalid TypeScript version range ${JSON.stringify(range)}: ${component}`,
    )
    this.name = 'TypeScriptVersionRangeError'
  }
}

function partial(text: string): PartialVersion | undefined {
  const match =
    /^([x*0]|[1-9]\d*)(?:\.([x*0]|[1-9]\d*)(?:\.([x*0]|[1-9]\d*)(?:-([a-z0-9-.]+))?(?:\+([a-z0-9-.]+))?)?)?$/i.exec(
      text,
    )
  if (!match) return undefined
  const [, major, minor = '*', patch = '*', pre = '', build = ''] = match
  if (
    pre &&
    !/^(?:0|[1-9]\d*|[a-z-][a-z0-9-]*)(?:\.(?:0|[1-9]\d*|[a-z-][a-z0-9-]*))*$/i.test(
      pre,
    )
  )
    throw new TypeScriptVersionRangeError(text, 'prerelease')
  if (build && !/^[a-z0-9-]+(?:\.[a-z0-9-]+)*$/i.test(build))
    throw new TypeScriptVersionRangeError(text, 'build')
  return {
    major,
    minor,
    patch,
    version: [
      wildcard(major) ? 0 : Number(major),
      wildcard(major) || wildcard(minor) ? 0 : Number(minor),
      wildcard(major) || wildcard(minor) || wildcard(patch) ? 0 : Number(patch),
      pre ? pre.split('.') : [],
    ],
  }
}

function increment(
  value: Version,
  field: 0 | 1 | 2,
  prerelease = false,
): Version {
  return [
    value[0] + Number(field === 0),
    field === 0 ? 0 : value[1] + Number(field === 1),
    field < 2 ? 0 : value[2] + 1,
    prerelease ? ['0'] : [],
  ]
}

function compare(value: Version): number {
  return (
    compiler[0] - value[0] ||
    compiler[1] - value[1] ||
    compiler[2] - value[2] ||
    Number(value[3].length > 0)
  )
}
type Operator = '<' | '<=' | '>' | '>=' | '='
type Comparator = readonly [Operator, Version]
function test([operator, value]: Comparator): boolean {
  const order = compare(value)
  switch (operator) {
    case '<':
      return order < 0
    case '<=':
      return order <= 0
    case '>':
      return order > 0
    case '>=':
      return order >= 0
    case '=':
      return order === 0
  }
}

function comparators(text: string): readonly Comparator[] | undefined {
  const hyphen = /^\s*([a-z0-9-+.*]+)\s+-\s+([a-z0-9-+.*]+)\s*$/i.exec(text)
  const result: Comparator[] = []
  if (hyphen) {
    const left = partial(hyphen[1])
    const right = partial(hyphen[2])
    if (!left || !right) return undefined
    if (!wildcard(left.major)) result.push(['>=', left.version])
    if (!wildcard(right.major))
      result.push(
        wildcard(right.minor)
          ? ['<', increment(right.version, 0)]
          : wildcard(right.patch)
            ? ['<', increment(right.version, 1)]
            : ['<=', right.version],
      )
    return result
  }
  for (const simple of text.split(/\s+/)) {
    const match = /^([~^<>=]|<=|>=)?\s*([a-z0-9-+.*]+)$/i.exec(simple.trim())
    if (!match) return undefined
    const value = partial(match[2])
    if (!value) return undefined
    const { major, minor, patch, version } = value
    const operator = match[1]
    if (wildcard(major)) {
      if (operator === '<' || operator === '>')
        result.push(['<', [0, 0, 0, ['0']]])
      continue
    }
    switch (operator) {
      case '~':
        result.push(
          ['>=', version],
          ['<', increment(version, wildcard(minor) ? 0 : 1)],
        )
        break
      case '^':
        result.push(
          ['>=', version],
          [
            '<',
            increment(
              version,
              version[0] > 0 || wildcard(minor)
                ? 0
                : version[1] > 0 || wildcard(patch)
                  ? 1
                  : 2,
            ),
          ],
        )
        break
      case '<':
      case '>=':
        result.push([
          operator,
          wildcard(minor) || wildcard(patch)
            ? [version[0], version[1], version[2], ['0']]
            : version,
        ])
        break
      case '<=':
      case '>':
        result.push(
          wildcard(minor) || wildcard(patch)
            ? [
                operator === '<=' ? '<' : '>=',
                increment(version, wildcard(minor) ? 0 : 1, true),
              ]
            : [operator, version],
        )
        break
      case '=':
      case undefined:
        if (wildcard(minor) || wildcard(patch))
          result.push(
            ['>=', [version[0], version[1], version[2], ['0']]],
            ['<', increment(version, wildcard(minor) ? 0 : 1, true)],
          )
        else result.push(['=', version])
        break
    }
  }
  return result
}

export function matchesTypeScriptVersion(range: string): boolean {
  const alternatives = range
    .trim()
    .split(/\|\|/)
    .filter(Boolean)
    .map((part) => comparators(part.trim()))
  return (
    alternatives.every((part) => part !== undefined) &&
    (alternatives.length === 0 ||
      alternatives.some((part) => part?.every(test)))
  )
}
