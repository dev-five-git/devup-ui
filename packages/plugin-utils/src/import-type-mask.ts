/** Receives already-masked code; only type-context import keywords are blanked. */
export function maskImportTypes(source: string): string {
  const tokens = [
    ...source.matchAll(
      /(['"])[^'"]*\1|=>|\?\.|===|!==|==|!=|&&|\|\||\?\?|[\w$]+|[^\s]/g,
    ),
  ]
  const output = source.split('')
  const pairs = new Map<number, number>()
  const stack: number[] = []
  const questions = new Map<number, number>()
  const ternaryColons = new Set<number>()
  const parameters = new Set<number>()
  const parameterEnds = new Set<number>()
  const classes = new Set<number>()
  const bindings = new Set<number>()
  const variables = new Set<number>()
  const value = (index: number): string => tokens[index]?.[0] ?? ''
  for (let index = 0; index < tokens.length; index += 1) {
    const word = value(index)
    if (/^[({[]$/.test(word)) stack.push(index)
    else if (/^[)}\]]$/.test(word)) {
      const opening = stack.pop()
      if (opening !== undefined) pairs.set(opening, index)
    }
    const scope = stack.at(-1) ?? -1
    if (word === '?' && value(index + 1) !== ':')
      questions.set(scope, (questions.get(scope) ?? 0) + 1)
    if (word === ':' && (questions.get(scope) ?? 0) > 0) {
      ternaryColons.add(index)
      questions.set(scope, (questions.get(scope) ?? 0) - 1)
    }
  }
  function typeEnd(start: number, stops: RegExp): number {
    const closing: string[] = []
    let functionType = /^(?:\(|<)$/.test(value(start))
    let conditional = false
    const delimiters: Readonly<Record<string, string>> = {
      '(': ')',
      '[': ']',
      '{': '}',
      '<': '>',
    }
    for (let cursor = start; cursor < tokens.length; cursor += 1) {
      const word = value(cursor)
      if (closing.length === 0) {
        if (
          cursor > start &&
          /[\r\n]/.test(
            source.slice(tokens[cursor - 1]?.index, tokens[cursor]?.index),
          ) &&
          /^(?:[\w$]+|\)|\]|\}|>|`|(['"])[^'"]*\1)$/.test(value(cursor - 1)) &&
          !/^(?:typeof|keyof|extends|infer|import|new|abstract|is|in)$/.test(
            value(cursor - 1),
          ) &&
          ((/^[A-Za-z_$][\w$]*$/.test(word) &&
            !/^(?:extends|is|in)$/.test(word)) ||
            (word === '(' && !(functionType && value(cursor - 1) === '>')))
        )
          return cursor
        if (word === 'extends') conditional = true
        if (word === '=>' && functionType && value(cursor - 1) === ')') {
          functionType = false
          continue
        }
        if (
          (stops.test(word) && !(word === '?' && conditional)) ||
          /^(?:const|let|var|export|function|class|interface)$/.test(word)
        )
          return cursor
      }
      if (
        word === '{' &&
        closing.length === 0 &&
        cursor > start &&
        !/^(?:[|&]|=>)$/.test(value(cursor - 1))
      )
        return cursor
      const delimiter = delimiters[word]
      if (delimiter) closing.push(delimiter)
      else if (word === closing.at(-1)) closing.pop()
      else if (/^[)}\]>]$/.test(word) && closing.length === 0) return cursor
    }
    return tokens.length
  }
  for (const [opening, closing] of pairs) {
    if (value(opening) !== '(') continue
    let prefix = opening - 1
    if (value(prefix) === '>') {
      let depth = 1
      while (prefix > 0 && depth > 0) {
        prefix -= 1
        if (value(prefix) === '>') depth += 1
        if (value(prefix) === '<') depth -= 1
      }
      prefix -= 1
    }
    const declaration =
      value(prefix) === 'function' ||
      value(prefix - 1) === 'function' ||
      (value(prefix - 1) === '*' && value(prefix - 2) === 'function')
    // A colon after grouping may belong to a ternary, not a return type.
    // Require the annotation to end at an arrow/body, or a function signature.
    const annotationEnd =
      value(closing + 1) === ':' && !ternaryColons.has(closing + 1)
        ? value(typeEnd(closing + 2, /^(?:=>|;|=|,|\?)$/))
        : ''
    if (
      (/^(?:=>|\{)$/.test(value(closing + 1)) ||
        /^(?:=>|\{)$/.test(annotationEnd) ||
        (value(closing + 1) === ':' &&
          declaration &&
          !ternaryColons.has(closing + 1))) &&
      !/^(?:if|while|for|with|switch|catch|import|require)$/.test(
        value(opening - 1),
      )
    ) {
      parameters.add(opening)
      parameterEnds.add(closing)
    }
  }
  function mask(start: number, end: number): void {
    for (const token of tokens.slice(start, end)) {
      if (token[0] !== 'import') continue
      for (let offset = token.index; offset < token.index + 6; offset += 1)
        output[offset] = ' '
    }
  }
  for (let index = 0; index < tokens.length; index += 1) {
    const word = value(index)
    const scope = stack.at(-1) ?? -1
    if (word === '<') {
      const end = typeEnd(index + 1, /^(?:>|;)$/)
      const operand = /^[A-Za-z_$][\w$]*$/.test(value(index - 1))
      const prefix =
        /^(?:=|\(|,|return|=>)$/.test(value(index - 1)) &&
        (value(index + 1) === 'import' ||
          /^(?:const|extends|,|=)$/.test(value(index + 2)))
      if (
        value(end) === '>' &&
        (prefix || (operand && /^(?:\(|\{|;|,|\))$/.test(value(end + 1))))
      ) {
        mask(index + 1, end)
        index = end
        continue
      }
    }
    if (word === 'type' && /^[A-Za-z_$][\w$]*$/.test(value(index + 1))) {
      const equals = typeEnd(index + 2, /^(?:=|;)$/)
      if (value(equals) === '=') {
        const end = typeEnd(equals + 1, /^;$/)
        mask(equals + 1, end)
        index = end - 1
        continue
      }
    }
    if (
      /^(?:interface|class)$/.test(word) &&
      value(index - 1) !== '.' &&
      /^(?:[A-Za-z_$][\w$]*|\{)$/.test(value(index + 1))
    ) {
      let body = index + 1
      while (body < tokens.length && !/^(?:\{|;)$/.test(value(body)))
        body = (pairs.get(body) ?? body) + 1
      const end = pairs.get(body)
      if (word === 'interface' && end !== undefined) {
        mask(body, end)
        index = end
        continue
      }
      if (word === 'class') classes.add(body)
    }
    if (
      /^(?:const|let|var)$/.test(word) &&
      /^(?:[A-Za-z_$][\w$]*|\{|\[)$/.test(value(index + 1))
    ) {
      variables.add(scope)
      bindings.add(scope)
    }
    if (
      (word === ':' && (bindings.has(scope) || parameterEnds.has(index - 1))) ||
      (/^(?:as|satisfies)$/.test(word) &&
        /^(?:[\w$]+|[)\]}!]|(['"])[^'"]*\1|`)$/.test(value(index - 1)) &&
        !/^(?:=|:)$/.test(value(index + 1)))
    ) {
      const end = typeEnd(index + 1, /^(?:=|,|;|=>|\+|-|&&|\|\||\?\?|\?)$/)
      mask(index + 1, end)
      index = end - 1
      continue
    }
    if (word === '=') bindings.delete(scope)
    if (word === ',' && (parameters.has(scope) || variables.has(scope)))
      bindings.add(scope)
    if (word === ';') {
      variables.delete(scope)
      if (classes.has(scope)) bindings.add(scope)
      else bindings.delete(scope)
    }
    if (/^[({[]$/.test(word)) {
      stack.push(index)
      if (parameters.has(index) || classes.has(index)) bindings.add(index)
    } else if (/^[)}\]]$/.test(word)) stack.pop()
  }
  return output.join('')
}
