export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function parseJsonc(source: string, numericLiterals = false): unknown {
  let text = ''
  let quoted = false
  for (let index = 0; index < source.length; index += 1) {
    const char = source[index]
    const next = source[index + 1]
    if (quoted) {
      text += char
      if (char === '\\') text += source[++index] ?? ''
      else if (char === '"') quoted = false
    } else if (char === '"') {
      quoted = true
      text += char
    } else if (char === '/' && next === '/') {
      while (index < source.length && source[index] !== '\n') index += 1
      text += '\n'
    } else if (char === '/' && next === '*') {
      const end = source.indexOf('*/', index + 2)
      if (end === -1) throw new SyntaxError('Unterminated JSON comment')
      text += ' '
      index = end + 1
    } else text += char
  }
  let json = ''
  quoted = false
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index]
    if (quoted) {
      json += char
      if (char === '\\') json += text[++index] ?? ''
      else if (char === '"') quoted = false
    } else if (char === '"') {
      quoted = true
      json += char
    } else if (numericLiterals && /[\d.]/.test(char)) {
      const numeric =
        /^(?:0[xX][\da-fA-F](?:_?[\da-fA-F])*|0[bB][01](?:_?[01])*|0[oO][0-7](?:_?[0-7])*|(?:0|[1-9][\d_]*)(?:\.[\d_]*)?(?:[eE][+-]?[\d_]+)?|\.[\d_]+(?:[eE][+-]?[\d_]+)?)/.exec(
          text.slice(index),
        )?.[0]
      if (!numeric || /^_|_$|__|_[.eE]|[.eE]_/.test(numeric))
        throw new SyntaxError('Invalid numeric literal')
      const value = Number(numeric.replaceAll('_', ''))
      // TS 6.0.3:42867-42873 keeps infinity; 1e999 is its valid JSON number spelling.
      json += value === Infinity ? '1e999' : String(value)
      index += numeric.length - 1
    } else if (char !== ',' || !/^\s*[}\]]/.test(text.slice(index + 1)))
      json += char
  }
  return JSON.parse(json.replace(/^\uFEFF/, ''))
}
