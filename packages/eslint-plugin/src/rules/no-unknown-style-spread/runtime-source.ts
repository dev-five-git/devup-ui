import {
  AST_NODE_TYPES,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

export function findBinding(
  source: TSESLint.SourceCode,
  node: TSESTree.Node,
  name: string,
): TSESLint.Scope.Variable | undefined {
  for (
    let scope: TSESLint.Scope.Scope | null = source.getScope(node);
    scope;
    scope = scope.upper
  ) {
    const variable = scope.set.get(name)
    if (variable) return variable
  }
  return undefined
}

/** A proof of runtime origin, deliberately not an evaluator of build-known code. */
export function runtimeSource(source: TSESLint.SourceCode) {
  const visiting = new Set<TSESLint.Scope.Variable>()
  const unknown = (node: TSESTree.Node): boolean => {
    switch (node.type) {
      case AST_NODE_TYPES.ThisExpression:
        return true
      case AST_NODE_TYPES.TSAsExpression:
      case AST_NODE_TYPES.TSTypeAssertion:
      case AST_NODE_TYPES.TSSatisfiesExpression:
      case AST_NODE_TYPES.TSNonNullExpression:
      case AST_NODE_TYPES.ChainExpression:
        return unknown(node.expression)
      case AST_NODE_TYPES.MemberExpression:
        return unknown(node.object)
      case AST_NODE_TYPES.ConditionalExpression:
        return unknown(node.consequent) && unknown(node.alternate)
      case AST_NODE_TYPES.LogicalExpression:
        return (
          unknown(node.left) && (node.operator !== '&&' || unknown(node.right))
        )
      case AST_NODE_TYPES.ObjectExpression:
        return node.properties.some(
          (property) =>
            property.type === AST_NODE_TYPES.SpreadElement &&
            unknown(property.argument),
        )
      case AST_NODE_TYPES.Identifier: {
        const variable = findBinding(source, node, node.name)
        if (!variable || visiting.has(variable)) return false
        if (
          variable.references.some(
            (reference) => reference.isWrite() && !reference.init,
          )
        )
          return false
        const [definition] = variable.defs
        if (!definition)
          return node.name === 'arguments' && variable.scope.type === 'function'
        switch (definition.type) {
          case 'Parameter':
          case 'CatchClause':
            return true
          case 'Variable': {
            const init = definition.node.init
            if (!init) return false
            visiting.add(variable)
            const result = unknown(init)
            visiting.delete(variable)
            return result
          }
          default:
            return false
        }
      }
      // All other expressions might be build-evaluable, including calls/imports.
      default:
        return false
    }
  }
  return unknown
}
