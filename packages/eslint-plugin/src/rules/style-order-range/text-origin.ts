import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

export type CssText = {
  readonly text: string
  readonly offsets: readonly number[]
  readonly holes: ReadonlyMap<number, TSESTree.Node>
  readonly node: TSESTree.Node
}

export function sourceText(
  node: TSESTree.Literal | TSESTree.TemplateLiteral,
): CssText {
  let text = ''
  const offsets: number[] = []
  const holes = new Map<number, TSESTree.Node>()
  const append = (raw: string, start: number, cooked = true) => {
    for (let index = 0; index < raw.length; index++) {
      const offset = start + index
      let character = raw[index]
      if (character === '\\' && cooked) {
        character = raw[++index]
        if (character === '\r' || character === '\n') {
          if (character === '\r' && raw[index + 1] === '\n') index++
          continue
        }
        if (character === 'u' || character === 'x') {
          const braced = raw[index + 1] === '{'
          const end = braced
            ? raw.indexOf('}', index + 2)
            : index + (character === 'u' ? 4 : 2)
          const digits = raw.slice(
            index + (braced ? 2 : 1),
            end + (braced ? 0 : 1),
          )
          character = String.fromCodePoint(Number.parseInt(digits, 16))
          index = end
        } else {
          const escapes: Readonly<Record<string, string>> = {
            n: '\n',
            r: '\r',
            t: '\t',
            b: '\b',
            f: '\f',
            v: '\v',
            '0': '\0',
          }
          character = escapes[character] ?? character
        }
      } else if (character === '\r') {
        character = '\n'
        if (raw[index + 1] === '\n') index++
      }
      text += character
      for (let unit = 0; unit < character.length; unit++) offsets.push(offset)
    }
  }
  if (node.type === AST_NODE_TYPES.Literal) {
    append(node.raw.slice(1, -1), node.range[0] + 1)
  } else {
    for (const [index, quasi] of node.quasis.entries()) {
      append(quasi.value.raw, quasi.range[0] + 1, quasi.value.cooked !== null)
      const expression = node.expressions[index]
      if (expression) {
        holes.set(text.length, expression)
        text += '\ufffc'
        offsets.push(expression.range[0])
      }
    }
  }
  offsets.push(node.range[1] - 1)
  return { text, offsets, holes, node }
}
