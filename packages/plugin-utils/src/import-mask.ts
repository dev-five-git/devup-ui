import { maskImportTypes } from './import-type-mask'

const typeParameterPrefix =
  /^<(?:\s|\/\*(?:[^*]|\*(?!\/))*\*\/|\/\/[^\r\n]*(?:\r?\n|$))*(?:const(?:\s|\/\*(?:[^*]|\*(?!\/))*\*\/|\/\/[^\r\n]*(?:\r?\n|$))+)?[A-Za-z_$][\w$]*(?:\s|\/\*(?:[^*]|\*(?!\/))*\*\/|\/\/[^\r\n]*(?:\r?\n|$))*(?:,|=|extends\b(?:\s|\/\*(?:[^*]|\*(?!\/))*\*\/|\/\/[^\r\n]*(?:\r?\n|$))+[^\s=>/])/

function startsExpression(previous: string): boolean {
  return (
    !/^(?:value|[\w$]+|\)|\]|\+\+|--|\.|\?\.)$/.test(previous) ||
    /^(?:return|throw|case|delete|void|typeof|new|yield|await|in|instanceof|of|else|do)$/.test(
      previous,
    )
  )
}

/** Lexical masking only: the bundler, not this scanner, validates syntax. */
export function maskImportText(
  source: string,
  jsx: boolean,
  typescript = true,
): string {
  const output = source.split('')
  let index = 0
  let previous = ''
  let jsxDepth = 0
  let text = false
  let tag = false
  let tagClosing = false
  const parentheses: boolean[] = []
  const expressions: {
    readonly braces: number
    readonly depth: number
    readonly tag: boolean
    readonly tagClosing: boolean
    readonly text: boolean
  }[] = []
  const templates: number[] = []
  const blocks = new Set<number>()
  const bodies: {
    readonly braces: number
    readonly parentheses: number
    readonly statement: boolean
  }[] = []
  let templateText = false
  let braces = 0
  function blank(start: number, end: number): void {
    for (
      let offset = start;
      offset < Math.min(end, source.length);
      offset += 1
    ) {
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
    }
    blank(start + 1, index - 1)
    previous = 'value'
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
    if (templateText) {
      blank(index, index + 1)
      index += 1
      if (char === '\\') {
        blank(index, index + 1)
        index += 1
      } else if (char === '`') {
        output[index - 1] = '`'
        templates.pop()
        templateText = false
        previous = 'value'
      } else if (char === '$' && next === '{') {
        blank(index, index + 1)
        index += 1
        braces += 1
        templateText = false
        previous = '('
      }
      continue
    }
    if ((tag || text) && char === '{') {
      expressions.push({ braces, depth: jsxDepth, tag, tagClosing, text })
      braces += 1
      tag = text = false
      index += 1
      previous = '{'
      continue
    }
    if (tag) {
      if (char === '"' || char === "'") {
        quoted(char)
        continue
      }
      if (char === '>') {
        jsxDepth += tagClosing ? -1 : source[index - 1] === '/' ? 0 : 1
        tag = false
        text = jsxDepth > (expressions.at(-1)?.depth ?? 0)
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
    if (char === '`') {
      templates.push(braces)
      templateText = true
      index += 1
      continue
    }
    if (char === '"' || char === "'") {
      quoted(char)
      continue
    }
    if (char === '/' && startsExpression(previous)) {
      regexLiteral()
      continue
    }
    const operator = text
      ? undefined
      : /^(?:>>>=|>>>|>>=|<<=|===|!==|\*\*=|&&=|\|\|=|\?\?=|=>|<<|>>|<=|>=|==|!=|&&|\|\||\?\?|\*\*|\+\+|--|\?\.|[+*/%&|^!-]=)/.exec(
          source.slice(index),
        )?.[0]
    if (operator) {
      previous = operator
      index += operator.length
      continue
    }
    if (
      jsx &&
      char === '<' &&
      (text ||
        (startsExpression(previous) &&
          !(typescript && typeParameterPrefix.test(source.slice(index))))) &&
      /[A-Za-z/>]/.test(next ?? '')
    ) {
      tag = true
      tagClosing = next === '/'
      text = false
      blank(index, index + 1)
      index += 1
      continue
    }
    if (char === '{') {
      const body = bodies.at(-1)
      if (body?.braces === braces && body.parentheses === parentheses.length) {
        bodies.pop()
        if (body.statement) blocks.add(braces)
      } else if (/^(?:|;|\)|=>|try|finally|else|do)$/.test(previous))
        blocks.add(braces)
      braces += 1
    }
    if (char === '}') {
      braces -= 1
      if (templates.at(-1) === braces) {
        templateText = true
        blank(index, index + 1)
      }
      const expression = expressions.at(-1)
      if (expression?.braces === braces) {
        expressions.pop()
        tag = expression.tag
        tagClosing = expression.tagClosing
        text = expression.text
      }
      previous = blocks.delete(braces) ? '' : 'value'
      index += 1
      continue
    }
    if (/[A-Za-z_$]/.test(char)) {
      const start = index++
      while (/[\w$]/.test(source[index] ?? '') && index < source.length)
        index += 1
      const word = source.slice(start, index)
      if (
        word === 'async' &&
        /^(?:\s|\/\*(?:[^*]|\*(?!\/))*\*\/)+function\b/.test(
          source.slice(index),
        )
      )
        continue
      if (word === 'function' || word === 'class')
        bodies.push({
          braces,
          parentheses: parentheses.length,
          statement:
            /^(?:|;|[\w$]+|\)|\]|\+\+|--)$/.test(previous) &&
            !/^(?:return|throw|case|delete|void|typeof|new|yield|await|in|instanceof|of|extends)$/.test(
              previous,
            ),
        })
      previous = word === 'await' && previous === 'for' ? 'for' : word
      continue
    }
    if (char === '(')
      parentheses.push(/^(?:if|while|for|with|switch|catch)$/.test(previous))
    if (!/\s/.test(char))
      previous =
        char === ')' && parentheses.pop()
          ? ''
          : char === '!' && typescript && !startsExpression(previous)
            ? 'value'
            : char
    index += 1
  }
  const masked = output.join('')
  return typescript ? maskImportTypes(masked) : masked
}
