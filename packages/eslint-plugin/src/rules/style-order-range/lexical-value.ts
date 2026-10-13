import {
  AST_NODE_TYPES,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

export type ScopeOf = (node: TSESTree.Node) => TSESLint.Scope.Scope

export function variableOf(
  node: TSESTree.Identifier | TSESTree.JSXIdentifier,
  scopeOf: ScopeOf,
): TSESLint.Scope.Variable | undefined {
  for (
    let scope: TSESLint.Scope.Scope | null = scopeOf(node);
    scope;
    scope = scope.upper
  ) {
    const variable = scope.set.get(node.name)
    if (variable) return variable
  }
  return undefined
}

export function unwrap(node: TSESTree.Node): TSESTree.Node {
  switch (node.type) {
    case AST_NODE_TYPES.TSAsExpression:
    case AST_NODE_TYPES.TSSatisfiesExpression:
    case AST_NODE_TYPES.TSNonNullExpression:
    case AST_NODE_TYPES.TSTypeAssertion:
      return unwrap(node.expression)
    default:
      return node
  }
}

export function constant(
  node: TSESTree.Identifier | TSESTree.JSXIdentifier,
  scopeOf: ScopeOf,
) {
  const variable = variableOf(node, scopeOf)
  const definition = variable?.defs[0]
  if (
    definition?.type !== 'Variable' ||
    definition.parent.kind !== 'const' ||
    variable?.references.some(
      (reference) => reference.isWrite() && !reference.init,
    )
  )
    return null
  return definition.node.init
}
