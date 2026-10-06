import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

export function propertyKeyName(node: TSESTree.Property): string | undefined {
  if (node.key.type === AST_NODE_TYPES.Identifier && !node.computed)
    return node.key.name
  if (
    node.key.type === AST_NODE_TYPES.Literal &&
    typeof node.key.value === 'string'
  )
    return node.key.value
}
