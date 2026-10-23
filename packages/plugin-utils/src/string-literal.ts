const escapes: Readonly<Record<string, string>> = {
  b: '\b',
  f: '\f',
  n: '\n',
  r: '\r',
  t: '\t',
  v: '\v',
}

/** Decode an already-tokenized quoted string without evaluating source. */
export function decodeStringLiteral(literal: string): string | undefined {
  const source = literal.slice(1, -1)
  let result = ''
  for (let index = 0; index < source.length; index++) {
    const char = source.charAt(index)
    if (char !== '\\') {
      if (char === '\r' || char === '\n') return undefined
      result += char
      continue
    }
    index++
    const escape = source.charAt(index)
    if (escape === '') return undefined
    if (escape === 'x' || escape === 'u') {
      const braced = escape === 'u' && source.charAt(index + 1) === '{'
      const start = index + (braced ? 2 : 1)
      const end = braced
        ? source.indexOf('}', start)
        : start + (escape === 'x' ? 2 : 4)
      const hex = source.slice(start, end)
      if (end < 0 || end > source.length || !/^[\da-f]+$/i.test(hex))
        return undefined
      const value = Number.parseInt(hex, 16)
      if (value > 0x10ffff) return undefined
      result += String.fromCodePoint(value)
      index = braced ? end : end - 1
      continue
    }
    if (escape === '\r') {
      if (source.charAt(index + 1) === '\n') index++
      continue
    }
    if (escape === '\n' || escape === '\u2028' || escape === '\u2029') continue
    if (/[0-7]/.test(escape)) {
      let octal = escape
      while (
        octal.length < (escape <= '3' ? 3 : 2) &&
        /[0-7]/.test(source.charAt(index + 1))
      ) {
        octal += source.charAt(++index)
      }
      result += String.fromCharCode(Number.parseInt(octal, 8))
      continue
    }
    result += escapes[escape] ?? escape
  }
  return result
}
