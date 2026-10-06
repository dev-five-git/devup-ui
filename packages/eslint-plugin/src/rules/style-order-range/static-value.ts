import {
  AST_NODE_TYPES,
  ASTUtils,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

import { constant, type ScopeOf, unwrap, variableOf } from './lexical-value'
import {
  binary,
  EXACT_MATH,
  primitive,
  type StaticValue,
} from './primitive-value'

export { constant, type ScopeOf, unwrap, variableOf } from './lexical-value'
export { EXACT_MATH, type StaticValue, validOrder } from './primitive-value'

/** Evaluate primitives only: no user code, getters, objects or arbitrary calls run. */
export function staticValue(
  input: TSESTree.Node,
  scopeOf: ScopeOf,
  evaluation: {
    readonly seen: ReadonlySet<TSESTree.Node>
    readonly arguments: ReadonlyMap<TSESLint.Scope.Variable, StaticValue>
    readonly allowUnknownArguments?: boolean
  } = { seen: new Set(), arguments: new Map() },
): StaticValue {
  const node = unwrap(input)
  if (evaluation.seen.has(node)) return null
  const next = { ...evaluation, seen: new Set(evaluation.seen).add(node) }
  const read = (value: TSESTree.Node) => staticValue(value, scopeOf, next)
  switch (node.type) {
    case AST_NODE_TYPES.Literal:
      return 'regex' in node ? null : { value: node.value }
    case AST_NODE_TYPES.Identifier: {
      const variable = variableOf(node, scopeOf)
      if (variable && evaluation.arguments.has(variable))
        return evaluation.arguments.get(variable) ?? null
      const init = constant(node, scopeOf)
      if (init) return read(init)
      if (variableOf(node, scopeOf)?.defs.length) return null
      switch (node.name) {
        case 'undefined':
          return { value: undefined }
        case 'NaN':
          return { value: NaN }
        case 'Infinity':
          return { value: Infinity }
        default:
          return null
      }
    }
    case AST_NODE_TYPES.TemplateLiteral: {
      let value = node.quasis[0].value.cooked
      if (value === null) return null
      for (let index = 0; index < node.expressions.length; index++) {
        const part = read(node.expressions[index])
        const tail = node.quasis[index + 1].value.cooked
        if (!part || tail === null) return null
        value += String(part.value) + tail
      }
      return { value }
    }
    case AST_NODE_TYPES.UnaryExpression: {
      const argument = read(node.argument)
      if (!argument) return null
      const value = argument.value
      switch (node.operator) {
        case '+':
          return typeof value === 'bigint' ? null : { value: Number(value) }
        case '-':
          return { value: typeof value === 'bigint' ? -value : -Number(value) }
        case '~':
          return { value: typeof value === 'bigint' ? ~value : ~Number(value) }
        case '!':
          return { value: !value }
        case 'void':
          return { value: undefined }
        case 'typeof':
          return { value: typeof value }
        default:
          return null
      }
    }
    case AST_NODE_TYPES.BinaryExpression: {
      const left = read(node.left)
      const right = read(node.right)
      if (!left || !right) return null
      return binary(node.operator, left.value, right.value)
    }
    case AST_NODE_TYPES.ConditionalExpression: {
      const test = read(node.test)
      return test ? read(test.value ? node.consequent : node.alternate) : null
    }
    case AST_NODE_TYPES.LogicalExpression: {
      const left = read(node.left)
      if (!left) return null
      if (node.operator === '&&') return left.value ? read(node.right) : left
      if (node.operator === '||') return left.value ? left : read(node.right)
      return left.value === null || left.value === undefined
        ? read(node.right)
        : left
    }
    case AST_NODE_TYPES.CallExpression: {
      const callee = unwrap(node.callee)
      if (
        callee.type === AST_NODE_TYPES.Identifier &&
        variableOf(callee, scopeOf)?.references.some(
          (reference) => reference.isWrite() && !reference.init,
        )
      )
        return null
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
        fn &&
        (fn.type === AST_NODE_TYPES.FunctionDeclaration ||
          fn.type === AST_NODE_TYPES.FunctionExpression ||
          fn.type === AST_NODE_TYPES.ArrowFunctionExpression)
      ) {
        if (
          fn.async ||
          fn.generator ||
          fn.params.length > node.arguments.length ||
          next.seen.has(fn)
        )
          return null
        const values = node.arguments.map(read)
        if (
          !evaluation.allowUnknownArguments &&
          values.some((value) => value === null)
        )
          return null
        const argumentsMap = new Map(evaluation.arguments)
        for (const [index, parameter] of fn.params.entries()) {
          if (parameter.type !== AST_NODE_TYPES.Identifier) return null
          const variable = variableOf(parameter, scopeOf)
          if (variable) argumentsMap.set(variable, values[index])
        }
        const body = fn.body
        if (!body) return null
        let result: TSESTree.Node = body
        if (body.type === AST_NODE_TYPES.BlockStatement) {
          const statements = body.body
          const last = statements[statements.length - 1]
          if (last?.type !== AST_NODE_TYPES.ReturnStatement || !last.argument)
            return null
          if (
            !statements.slice(0, -1).every(
              (statement) =>
                statement.type === AST_NODE_TYPES.VariableDeclaration &&
                statement.kind === 'const' &&
                statement.declarations.every(
                  (declaration) =>
                    declaration.id.type === AST_NODE_TYPES.Identifier &&
                    declaration.init !== null &&
                    staticValue(declaration.init, scopeOf, {
                      ...next,
                      arguments: argumentsMap,
                    }) !== null,
                ),
            )
          )
            return null
          result = last.argument
        }
        return staticValue(result, scopeOf, {
          seen: new Set(next.seen).add(fn),
          arguments: argumentsMap,
        })
      }
      if (node.callee.type === AST_NODE_TYPES.MemberExpression) {
        const method = node.callee
        const name = method.computed
          ? read(method.property)
          : method.property.type === AST_NODE_TYPES.Identifier
            ? { value: method.property.name }
            : null
        if (
          method.object.type !== AST_NODE_TYPES.Identifier ||
          method.object.name !== 'Math' ||
          variableOf(method.object, scopeOf)?.defs.length ||
          typeof name?.value !== 'string' ||
          !EXACT_MATH.has(name.value)
        )
          return null
        return primitive(ASTUtils.getStaticValue(node, scopeOf(node)))
      }
      if (
        node.callee.type !== AST_NODE_TYPES.Identifier ||
        variableOf(node.callee, scopeOf)?.defs.length ||
        node.arguments.length !== 1
      )
        return null
      const argument = read(node.arguments[0])
      if (!argument) return null
      switch (node.callee.name) {
        case 'Number':
          return { value: Number(argument.value) }
        case 'String':
          return { value: String(argument.value) }
        case 'Boolean':
          return { value: Boolean(argument.value) }
        default:
          return null
      }
    }
    case AST_NODE_TYPES.MemberExpression:
      return primitive(ASTUtils.getStaticValue(node, scopeOf(node)))
    default:
      return null
  }
}
