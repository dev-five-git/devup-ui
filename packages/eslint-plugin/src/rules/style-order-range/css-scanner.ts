import type { TSESTree } from '@typescript-eslint/utils'

import type { Mode } from './api-context'
import type { CssText } from './text-origin'

export type Directive = {
  readonly key: number
  readonly keyEnd: number
  readonly start: number
  readonly end: number
  readonly mode: Mode
}

/** Structural delimiters are read before comments, escapes or values are decoded. */
export function directives(source: CssText, initial: Mode) {
  const { text, holes } = source
  const result: Directive[] = []
  const visibleHoles = new Set<number>()
  const mixins: { readonly node: TSESTree.Node; readonly mode: Mode }[] = []
  let index = 0
  const scan = (mode: Mode) => {
    let start = index
    let colon = -1
    let parentheses = 0
    let brackets = 0
    let customBraces = 0
    let quote = ''
    const declaration = (end: number) => {
      if (colon < 0) return
      const rawKey = trivia(text.slice(start, colon)).trim()
      const key = cssValue(`'${rawKey}'`)
      if (key === 'style-order' || key === 'styleOrder') {
        const keyStart = start + bounds(text.slice(start, colon))[0]
        result.push({
          key: keyStart,
          keyEnd: start + bounds(text.slice(start, colon))[1],
          start: colon + 1,
          end,
          mode,
        })
      }
    }
    while (index < text.length) {
      const character = text[index]
      const hole = holes.get(index)
      if (hole) {
        visibleHoles.add(index)
        if (
          !quote &&
          !parentheses &&
          !brackets &&
          !customBraces &&
          colon < 0 &&
          trivia(text.slice(start, index)).trim() === ''
        ) {
          mixins.push({ node: hole, mode })
          index++
          start = index
          continue
        }
        index++
        continue
      }
      if (character === '\\') {
        index += 2
        continue
      }
      if (quote) {
        if (character === quote) quote = ''
        index++
        continue
      }
      if (character === '"' || character === "'") {
        quote = character
        index++
        continue
      }
      if (character === '/' && text[index + 1] === '*') {
        const end = text.indexOf('*/', index + 2)
        index = end < 0 ? text.length : end + 2
        continue
      }
      if (character === '(') parentheses++
      if (character === ')') parentheses--
      if (character === '[') brackets++
      if (character === ']') brackets--
      if (parentheses || brackets) {
        index++
        continue
      }
      if (character === '{') {
        const prelude = trivia(text.slice(start, index)).trim()
        if (
          customBraces ||
          (colon >= 0 &&
            trivia(text.slice(start, colon)).trim().startsWith('--'))
        ) {
          customBraces++
          index++
          continue
        }
        index++
        const child = /^@(?:[\w-]*keyframes)\b/i.test(prelude)
          ? 'keyframes'
          : /^@font-face\b/i.test(prelude)
            ? 'fontface'
            : mode
        scan(child)
        start = index
        colon = -1
        continue
      }
      if (character === '}' && customBraces) {
        customBraces--
        index++
        continue
      }
      if (customBraces) {
        index++
        continue
      }
      if (character === ':' && colon < 0) colon = index
      if (character === ';' || character === '}') {
        declaration(index)
        index++
        start = index
        colon = -1
        if (character === '}') return
        continue
      }
      index++
    }
    declaration(index)
  }
  scan(initial)
  return { directives: result, mixins, visibleHoles }
}

export function bounds(text: string): readonly [number, number] {
  let start = 0
  let end = text.length
  while (start < end) {
    if (/\s/.test(text[start])) start++
    else if (text.slice(start, start + 2) === '/*') {
      const close = text.indexOf('*/', start + 2)
      start = close < 0 ? end : close + 2
    } else break
  }
  while (end > start) {
    if (/\s/.test(text[end - 1])) end--
    else if (text.slice(end - 2, end) === '*/') {
      const open = text.lastIndexOf('/*', end - 2)
      if (open < start) break
      end = open
    } else break
  }
  return [start, end]
}

export function trivia(
  text: string,
  initialQuote = '',
  separator = '',
): string {
  let result = ''
  let quote = initialQuote
  for (let index = 0; index < text.length; index++) {
    const character = text[index]
    if (character === '\\') {
      result += character + (text[++index] ?? '')
      continue
    }
    if (quote) {
      result += character
      if (character === quote) quote = ''
    } else if (character === '"' || character === "'") {
      quote = character
      result += character
    } else if (character === '/' && text[index + 1] === '*') {
      result += separator
      const end = text.indexOf('*/', index + 2)
      index = end < 0 ? text.length : end + 1
    } else result += character
  }
  return result
}

export function cssValue(text: string): string | number {
  const token = trivia(text, '', ' ').trim()
  if (/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i.test(token))
    return Number(token)
  if (
    token.length >= 2 &&
    (token[0] === '"' || token[0] === "'") &&
    token.at(-1) === token[0]
  ) {
    return token
      .slice(1, -1)
      .replace(
        /\\([\da-f]{1,6}[ \t\r\n\f]?|\r\n|[\s\S])/gi,
        (_, escape: string) => {
          if (/^[\da-f]/i.test(escape)) {
            const code = Number.parseInt(escape, 16)
            return code === 0 ||
              code > 0x10ffff ||
              (code >= 0xd800 && code <= 0xdfff)
              ? '\ufffd'
              : String.fromCodePoint(code)
          }
          return /^[\r\n\f]/.test(escape) ? '' : escape
        },
      )
  }
  return token
}
