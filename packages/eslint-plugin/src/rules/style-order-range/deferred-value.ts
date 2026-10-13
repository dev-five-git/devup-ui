import {
  AST_NODE_TYPES,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

import {
  constant,
  EXACT_MATH,
  type ScopeOf,
  staticValue,
  unwrap,
  validOrder,
  variableOf,
} from './static-value'

type Proof = {
  readonly seen: ReadonlySet<TSESTree.Node>
  readonly parameters: ReadonlySet<TSESLint.Scope.Variable>
}

export function deferredValue(input: TSESTree.Node, scopeOf: ScopeOf): boolean {
  if (!hasDeferredOrigin(input, scopeOf) || !canBuildRead(input, scopeOf))
    return false
  const result = staticValue(input, scopeOf, {
    seen: new Set(),
    arguments: new Map(),
    allowUnknownArguments: true,
  })
  return result === null || validOrder(result.value)
}

function hasDeferredOrigin(
  input: TSESTree.Node,
  scopeOf: ScopeOf,
  seen = new Set<TSESTree.Node>(),
): boolean {
  const node = unwrap(input)
  if (seen.has(node)) return false
  const next = new Set(seen).add(node)
  const read = (value: TSESTree.Node) => hasDeferredOrigin(value, scopeOf, next)
  switch (node.type) {
    case AST_NODE_TYPES.Identifier: {
      if (variableOf(node, scopeOf)?.defs[0]?.type === 'ImportBinding')
        return true
      const init = constant(node, scopeOf)
      return init !== null && read(init)
    }
    case AST_NODE_TYPES.CallExpression:
      return true
    case AST_NODE_TYPES.MemberExpression:
      return read(node.object) || (node.computed && read(node.property))
    case AST_NODE_TYPES.UnaryExpression:
      return read(node.argument)
    case AST_NODE_TYPES.BinaryExpression:
    case AST_NODE_TYPES.LogicalExpression:
      return read(node.left) || read(node.right)
    case AST_NODE_TYPES.ConditionalExpression:
      return read(node.test) || read(node.consequent) || read(node.alternate)
    case AST_NODE_TYPES.TemplateLiteral:
      return node.expressions.some(read)
    default:
      return false
  }
}

function canBuildRead(
  input: TSESTree.Node,
  scopeOf: ScopeOf,
  proof: Proof = { seen: new Set(), parameters: new Set() },
): boolean {
  const node = unwrap(input)
  if (proof.seen.has(node)) return false
  const next = { ...proof, seen: new Set(proof.seen).add(node) }
  const read = (value: TSESTree.Node) => canBuildRead(value, scopeOf, next)
  if (staticValue(node, scopeOf)) return true
  switch (node.type) {
    case AST_NODE_TYPES.Identifier: {
      const variable = variableOf(node, scopeOf)
      const definition = variable?.defs[0]
      if (variable && proof.parameters.has(variable)) return true
      if (definition?.type === 'ImportBinding')
        return !variable?.references.some((reference) => reference.isWrite())
      const init = constant(node, scopeOf)
      return init !== null && read(init)
    }
    case AST_NODE_TYPES.MemberExpression:
      return read(node.object) && (!node.computed || read(node.property))
    case AST_NODE_TYPES.UnaryExpression:
      return node.operator !== 'delete' && read(node.argument)
    case AST_NODE_TYPES.BinaryExpression:
      return node.operator !== '**' && read(node.left) && read(node.right)
    case AST_NODE_TYPES.ConditionalExpression:
      return read(node.test) && read(node.consequent) && read(node.alternate)
    case AST_NODE_TYPES.LogicalExpression:
      return read(node.left) && read(node.right)
    case AST_NODE_TYPES.TemplateLiteral:
      return node.expressions.every(read)
    case AST_NODE_TYPES.ArrayExpression:
      return node.elements.every((element) => element === null || read(element))
    case AST_NODE_TYPES.ObjectExpression:
      return node.properties.every(
        (property) =>
          property.type === AST_NODE_TYPES.Property &&
          property.kind === 'init' &&
          !property.method &&
          (!property.computed || read(property.key)) &&
          read(property.value),
      )
    case AST_NODE_TYPES.CallExpression: {
      if (!node.arguments.every(read)) return false
      const callee = unwrap(node.callee)
      if (
        callee.type === AST_NODE_TYPES.Identifier &&
        variableOf(callee, scopeOf)?.references.some(
          (reference) => reference.isWrite() && !reference.init,
        )
      )
        return false
      if (
        callee.type === AST_NODE_TYPES.Identifier &&
        !variableOf(callee, scopeOf)?.defs.length &&
        ['Number', 'String', 'Boolean'].includes(callee.name)
      )
        return node.arguments.length === 1
      if (
        callee.type === AST_NODE_TYPES.MemberExpression &&
        callee.object.type === AST_NODE_TYPES.Identifier &&
        callee.object.name === 'Math' &&
        !variableOf(callee.object, scopeOf)?.defs.length
      ) {
        const name = callee.computed
          ? staticValue(callee.property, scopeOf)
          : callee.property.type === AST_NODE_TYPES.Identifier
            ? { value: callee.property.name }
            : null
        return typeof name?.value === 'string' && EXACT_MATH.has(name.value)
      }
      const definition =
        callee.type === AST_NODE_TYPES.Identifier
          ? variableOf(callee, scopeOf)?.defs[0]
          : undefined
      const fn =
        definition?.type === 'FunctionName'
          ? definition.node
          : callee.type === AST_NODE_TYPES.Identifier
            ? constant(callee, scopeOf)
            : callee
      if (
        !fn ||
        (fn.type !== AST_NODE_TYPES.FunctionDeclaration &&
          fn.type !== AST_NODE_TYPES.FunctionExpression &&
          fn.type !== AST_NODE_TYPES.ArrowFunctionExpression)
      )
        return false
      if (fn.async || fn.generator || fn.params.length > node.arguments.length)
        return false
      const parameters = new Set(proof.parameters)
      for (const parameter of fn.params) {
        if (parameter.type !== AST_NODE_TYPES.Identifier) return false
        const variable = variableOf(parameter, scopeOf)
        if (variable) parameters.add(variable)
      }
      const body = fn.body
      return body?.type === AST_NODE_TYPES.BlockStatement
        ? body.body.every((statement) => {
            if (statement.type === AST_NODE_TYPES.ReturnStatement)
              return (
                statement.argument !== null &&
                canBuildRead(statement.argument, scopeOf, {
                  ...next,
                  parameters,
                })
              )
            return (
              statement.type === AST_NODE_TYPES.VariableDeclaration &&
              statement.kind === 'const' &&
              statement.declarations.every(
                (declaration) =>
                  declaration.init !== null &&
                  canBuildRead(declaration.init, scopeOf, {
                    ...next,
                    parameters,
                  }),
              )
            )
          })
        : body !== null &&
            body !== undefined &&
            canBuildRead(body, scopeOf, { ...next, parameters })
    }
    default:
      return false
  }
}
