import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import { keyOf } from './api-context'
import { constant, type ScopeOf, unwrap } from './static-value'

export function factorySettings(
  input: TSESTree.Node,
  scopeOf: ScopeOf,
  seen = new Set<TSESTree.Node>(),
): boolean {
  const node = unwrap(input)
  if (seen.has(node)) return false
  if (node.type === AST_NODE_TYPES.Identifier) {
    const init = constant(node, scopeOf)
    return (
      init !== null && factorySettings(init, scopeOf, new Set(seen).add(node))
    )
  }
  return (
    node.type === AST_NODE_TYPES.MemberExpression &&
    ['attrs', 'withConfig'].includes(keyOf(node, scopeOf) ?? '')
  )
}
