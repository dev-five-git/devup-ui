/** Lexical masking only: the bundler, not this scanner, validates syntax. */
export function maskImportText(source: string, jsx: boolean): string {
  const output = source.split('')
  let index = 0
  let previous = ''
  let jsxDepth = 0
  let text = false
  let tag = false
  let tagClosing = false
  const parentheses: boolean[] = []
  const expressionTags: boolean[] = []
  const expressions: number[] = []
  let braces = 0
  function blank(start: number, end: number): void {
    for (let offset = start; offset < end; offset += 1) {
      if (output[offset] !== '\n' && output[offset] !== '\r')
        output[offset] = ' '
    }
  }
  function quoted(quote: string): void {
    const start = index++
    while (index < source.length) {
      const char = source[index++]
      if (char === '\\') index += 1
      else if (char === quote) break
      else if (quote === '`' && char === '$' && source[index] === '{') {
        index += 1
        templateExpression()
      }
    }
    blank(start + (quote === '`' ? 0 : 1), index - (quote === '`' ? 0 : 1))
    previous = 'value'
  }
  function templateExpression(): void {
    let depth = 1
    previous = ''
    while (index < source.length && depth) {
      const char = source[index]
      if (char === '"' || char === "'" || char === '`') quoted(char)
      else if (char === '/' && source[index + 1] === '*') comment(false)
      else if (char === '/' && source[index + 1] === '/') comment(true)
      else if (char === '/' && /^(?:|=|\(|,|:|\{|return)$/.test(previous))
        regexLiteral()
      else {
        if (char === '{') depth += 1
        if (char === '}') depth -= 1
        if (!/\s/.test(char)) previous = char
        index += 1
      }
    }
  }
  function comment(line: boolean): void {
    const start = index
    index += 2
    while (
      index < source.length &&
      (line
        ? source[index] !== '\n'
        : !(source[index] === '*' && source[index + 1] === '/'))
    )
      index += 1
    if (!line) index += 2
    blank(start, Math.min(index, source.length))
  }
  function regexLiteral(): void {
    const start = index++
    let bracket = false
    while (index < source.length) {
      const part = source[index++]
      if (part === '\\') index += 1
      else if (part === '[') bracket = true
      else if (part === ']') bracket = false
      else if (part === '/' && !bracket) break
    }
    while (/[a-z]/i.test(source[index] ?? '') && index < source.length)
      index += 1
    blank(start, index)
    previous = 'value'
  }
  while (index < source.length) {
    const char = source[index]
    const next = source[index + 1]
    if (tag) {
      if (char === '"' || char === "'") {
        quoted(char)
        continue
      }
      if (char === '{') {
        expressions.push(braces)
        expressionTags.push(true)
        braces += 1
        tag = false
        index += 1
        previous = '{'
        continue
      }
      if (char === '>') {
        jsxDepth += tagClosing ? -1 : source[index - 1] === '/' ? 0 : 1
        tag = false
        text = jsxDepth > 0
        previous = 'value'
      }
      blank(index, index + 1)
      index += 1
      continue
    }
    if (text && char !== '<' && char !== '{') {
      blank(index, index + 1)
      index += 1
      continue
    }
    if (char === '/' && (next === '/' || next === '*')) {
      comment(next === '/')
      continue
    }
    if (char === '"' || char === "'" || char === '`') {
      quoted(char)
      continue
    }
    if (char === '/' && !/^(?:value|[\w$]+|\)|\])$/.test(previous)) {
      regexLiteral()
      continue
    }
    if (
      jsx &&
      char === '<' &&
      (text || /^(?:|=|\(|\[|,|:|\?|&|\||return|=>)$/.test(previous)) &&
      /[A-Za-z/>]/.test(next ?? '')
    ) {
      tag = true
      tagClosing = next === '/'
      text = false
      blank(index, index + 1)
      index += 1
      continue
    }
    if (text && char === '{') {
      expressions.push(braces)
      expressionTags.push(false)
      braces += 1
      text = false
      previous = '{'
      index += 1
      continue
    }
    if (char === '{') braces += 1
    if (char === '}') {
      braces -= 1
      if (expressions.at(-1) === braces) {
        expressions.pop()
        tag = expressionTags.pop() === true
        text = !tag && jsxDepth > 0
      }
    }
    if (/[A-Za-z_$]/.test(char)) {
      const start = index++
      while (/[\w$]/.test(source[index] ?? '') && index < source.length)
        index += 1
      const word = source.slice(start, index)
      previous =
        /^(?:return|throw|case|delete|void|typeof|new|yield|await)$/.test(word)
          ? ''
          : word
      continue
    }
    if (char === '(')
      parentheses.push(/^(?:if|while|for|with|switch|catch)$/.test(previous))
    if (!/\s/.test(char))
      previous =
        char === ')' && parentheses.pop()
          ? ''
          : char === '>' && previous === '='
            ? '=>'
            : char
    index += 1
  }
  return output.join('')
}
