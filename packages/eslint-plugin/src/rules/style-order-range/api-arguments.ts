import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import { constant, type ScopeOf, unwrap } from './static-value'

export function argumentsOf(
  args: readonly TSESTree.Node[],
  scopeOf: ScopeOf,
): TSESTree.Node[] {
  const expand = (
    input: TSESTree.Node,
    seen = new Set<TSESTree.Node>(),
  ): TSESTree.Node[] => {
    const node = unwrap(input)
    if (seen.has(node)) return []
    const next = new Set(seen).add(node)
    if (node.type === AST_NODE_TYPES.Identifier) {
      const init = constant(node, scopeOf)
      return init ? expand(init, next) : []
    }
    if (node.type !== AST_NODE_TYPES.ArrayExpression) return []
    return node.elements.flatMap((element) =>
      element === null
        ? []
        : element.type === AST_NODE_TYPES.SpreadElement
          ? expand(element.argument, next)
          : [element],
    )
  }
  return args.flatMap((argument) =>
    argument.type === AST_NODE_TYPES.SpreadElement
      ? expand(argument.argument)
      : [argument],
  )
}
