import { BUILTIN_SHORTHAND_TARGETS, CSS_PROPERTY_NAMES } from './property-names'

type KebabTail<S extends string> = S extends `${infer Head}${infer Tail}`
  ? `${Head extends Lowercase<Head> ? Head : `-${Lowercase<Head>}`}${KebabTail<Tail>}`
  : S
type Kebab<S extends string> =
  KebabTail<S> extends `-${infer Rest}` ? Rest : KebabTail<S>
type CssName = (typeof CSS_PROPERTY_NAMES)[number]
type CssKebabName<S extends string = CssName> = S extends
  `Webkit${string}` | `Moz${string}` | `ms${string}` | `O${string}`
  ? `-${Kebab<S>}`
  : Kebab<S>

export type CustomShorthandTarget =
  | CssName
  | CssKebabName
  | keyof typeof BUILTIN_SHORTHAND_TARGETS
  | `--${string}`

export interface ShorthandErrorLocation {
  readonly alias: string | null
  readonly index: number | null
  readonly target: unknown
}

export class ShorthandConfigError extends Error {
  readonly alias: string | null
  readonly index: number | null
  readonly target: unknown
  readonly path: string

  constructor(location: ShorthandErrorLocation, requirement: string) {
    const { alias, index, target } = location
    const path = `shorthands${alias === null ? '' : `[${JSON.stringify(alias)}]`}${index === null ? '' : `[${index}]`}`
    super(
      `devup-ui option ${path}: shorthand ${JSON.stringify(alias)} cannot use ${typeof target === 'string' ? JSON.stringify(target) : typeof target} at build time: ${requirement}`,
    )
    this.name = 'ShorthandConfigError'
    this.alias = alias
    this.index = index
    this.target = target
    this.path = path
  }
}

function kebabName(name: string): string {
  const prefix = /^(Webkit|Moz|ms|O)[A-Z]/.test(name) ? '-' : ''
  return (
    prefix +
    name.replace(
      /[A-Z]/g,
      (letter, index: number) =>
        `${index === 0 ? '' : '-'}${letter.toLowerCase()}`,
    )
  )
}

const TARGETS = new Map<string, readonly string[]>(
  CSS_PROPERTY_NAMES.flatMap((name) => {
    const canonical = kebabName(name)
    return [
      [name, [canonical]],
      [canonical, [canonical]],
    ] satisfies [string, readonly string[]][]
  }),
)
for (const [alias, targets] of Object.entries(BUILTIN_SHORTHAND_TARGETS)) {
  TARGETS.set(alias, targets)
}

export function isCustomShorthandTarget(
  target: unknown,
): target is CustomShorthandTarget {
  return (
    typeof target === 'string' &&
    (TARGETS.has(target) ||
      /^--[A-Za-z0-9_\-\u0080-\u{10FFFF}]+$/u.test(target))
  )
}

function isRecord(input: unknown): input is Record<string, unknown> {
  return typeof input === 'object' && input !== null && !Array.isArray(input)
}

function isTargets(targets: unknown): targets is readonly unknown[] {
  return Array.isArray(targets)
}

export function normalizeShorthands(
  input: unknown,
): Record<string, readonly string[]> {
  if (!isRecord(input)) {
    throw new ShorthandConfigError(
      { alias: null, index: null, target: input },
      'shorthands must be an object',
    )
  }
  const entries: [string, readonly string[]][] = []
  for (const [alias, targets] of Object.entries(input).sort(([a], [b]) =>
    a < b ? -1 : a > b ? 1 : 0,
  )) {
    if (!isTargets(targets)) {
      throw new ShorthandConfigError(
        { alias, index: null, target: targets },
        'its targets must be an array',
      )
    }
    const normalized: string[] = []
    for (const [index, target] of targets.entries()) {
      if (!isCustomShorthandTarget(target)) {
        throw new ShorthandConfigError(
          { alias, index, target },
          'its targets must be supported CSS properties, built-in aliases or custom properties',
        )
      }
      normalized.push(...(TARGETS.get(target) ?? [target]))
    }
    entries.push([alias, normalized])
  }
  return Object.fromEntries(entries)
}
