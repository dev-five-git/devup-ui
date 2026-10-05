import { maskImportText } from './import-mask'

export interface ImportReference {
  readonly kind: 'static' | 'dynamic'
  readonly specifier: string
}

function literalValue(raw: string): string {
  return raw
    .slice(1, -1)
    .replace(
      /\\(?:u\{([\da-f]+)\}|u([\da-f]{4})|x([\da-f]{2})|\r\n|[\s\S])/gi,
      (
        escape: string,
        point: string | undefined,
        unicode: string | undefined,
        hex: string | undefined,
      ) => {
        const digits = point ?? unicode ?? hex
        if (digits) return String.fromCodePoint(Number.parseInt(digits, 16))
        const char = escape.slice(1)
        const escaped: Readonly<Record<string, string>> = {
          n: '\n',
          r: '\r',
          t: '\t',
          b: '\b',
          f: '\f',
          v: '\v',
          '0': '\0',
        }
        return /[\r\n]/.test(char) ? '' : (escaped[char] ?? char)
      },
    )
}

function allInlineTypes(clause: readonly string[]): boolean {
  if (clause[0] !== '{' || clause.at(-1) !== '}') return false
  const specifiers = clause
    .slice(1, -1)
    .join(' ')
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean)
  return (
    specifiers.length > 0 &&
    specifiers.every((value) => {
      const names = value.split(/\s+/)
      return (
        names[0] === 'type' &&
        names.length > 1 &&
        (names[1] !== 'as' ||
          names.length === 2 ||
          (names[2] === 'as' && names.length > 3))
      )
    })
  )
}

/** Internal scanner seam; not part of the package export map. */
export function scanImports(
  source: string,
  jsx: boolean,
  typescript = true,
): ImportReference[] {
  const tokens = [
    ...maskImportText(source, jsx, typescript).matchAll(
      /(['"])[^'"]*\1|[\w$]+|[^\s]/g,
    ),
  ].map((match) => ({
    value: match[0],
    literal: match[1]
      ? literalValue(source.slice(match.index, match.index + match[0].length))
      : undefined,
  }))
  const imports: ImportReference[] = []
  for (let index = 0; index < tokens.length; index += 1) {
    const word = tokens[index].value
    if (
      !['import', 'export', 'require'].includes(word) ||
      tokens[index - 1]?.value === '.'
    )
      continue
    const next = tokens[index + 1]
    if (next?.value === '(' && word !== 'export') {
      const argument = tokens[index + 2]?.literal
      const end = tokens[index + 3]?.value
      if (argument && (end === ')' || (word === 'import' && end === ',')))
        imports.push({
          kind: word === 'require' ? 'static' : 'dynamic',
          specifier: argument,
        })
      continue
    }
    if (word === 'require') continue
    if (word === 'import' && next?.literal) {
      imports.push({ kind: 'static', specifier: next.literal })
      continue
    }
    const clause: string[] = []
    let bindings = 0
    for (let cursor = index + 1; cursor < tokens.length; cursor += 1) {
      const token = tokens[cursor]
      if (
        token.value === 'from' &&
        bindings === 0 &&
        tokens[cursor + 1]?.literal !== undefined
      ) {
        const specifier = tokens[cursor + 1]?.literal
        if (
          specifier &&
          !(clause[0] === 'type' && clause.length > 1 && clause[1] !== ',') &&
          !allInlineTypes(clause)
        )
          imports.push({ kind: 'static', specifier })
        break
      }
      if (
        !/^(?:[\w$]+|[*{},])$/.test(token.value) &&
        token.literal === undefined
      )
        break
      if (token.value === '{') bindings += 1
      if (token.value === '}') bindings -= 1
      clause.push(token.value)
    }
  }
  return imports
}
