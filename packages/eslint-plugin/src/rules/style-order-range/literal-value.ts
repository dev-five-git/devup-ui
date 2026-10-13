import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import {
  constant,
  type ScopeOf,
  type StaticValue,
  staticValue,
  unwrap,
  validOrder,
} from './static-value'

type Values =
  | readonly {
      readonly value: Exclude<StaticValue, null>['value']
      readonly node: TSESTree.Node
    }[]
  | null

function merge(left: Values, right: Values): Values {
  if (!left || !right) return null
  const unique = new Map<string, NonNullable<Values>[number]>()
  for (const value of [...left, ...right])
    unique.set(
      `${validOrder(value.value) ? 'order' : typeof value.value}:${String(value.value)}`,
      value,
    )
  return unique.size <= 256 ? [...unique.values()] : null
}

/** Enumerate only bounded primitive branch shapes; never execute callbacks. */
export function finiteValues(
  input: TSESTree.Node,
  scopeOf: ScopeOf,
  state: {
    readonly callback: boolean
    readonly seen: ReadonlySet<TSESTree.Node>
    readonly absence?: boolean
  } = { callback: false, seen: new Set() },
): Values {
  const node = unwrap(input)
  if (state.seen.has(node)) return null
  const next = { ...state, seen: new Set(state.seen).add(node) }
  const value = staticValue(node, scopeOf)
  if (value) return [{ value: value.value, node }]
  const read = (child: TSESTree.Node) => finiteValues(child, scopeOf, next)
  switch (node.type) {
    case AST_NODE_TYPES.Identifier: {
      const init = constant(node, scopeOf)
      return init ? read(init) : null
    }
    case AST_NODE_TYPES.ConditionalExpression: {
      const left = read(node.consequent)
      const right = read(node.alternate)
      return merge(left, right)
    }
    case AST_NODE_TYPES.LogicalExpression:
      return node.operator === '&&' && state.absence !== false
        ? read(node.right)
        : null
    case AST_NODE_TYPES.ArrowFunctionExpression:
    case AST_NODE_TYPES.FunctionExpression: {
      if (!state.callback || node.async || node.generator) return null
      if (node.body.type !== AST_NODE_TYPES.BlockStatement)
        return read(node.body)
      const sequence = (statements: readonly TSESTree.Node[]): Values => {
        const [statement, ...tail] = statements
        return statement ? returns(statement, tail) : null
      }
      const returns = (
        statement: TSESTree.Node,
        tail: readonly TSESTree.Node[],
      ): Values => {
        switch (statement.type) {
          case AST_NODE_TYPES.ReturnStatement:
            return statement.argument ? read(statement.argument) : null
          case AST_NODE_TYPES.VariableDeclaration:
            return statement.kind === 'const' ? sequence(tail) : null
          case AST_NODE_TYPES.BlockStatement:
            return sequence([...statement.body, ...tail])
          case AST_NODE_TYPES.IfStatement: {
            const test = staticValue(statement.test, scopeOf)
            const alternate = () =>
              statement.alternate
                ? returns(statement.alternate, tail)
                : sequence(tail)
            if (test)
              return test.value
                ? returns(statement.consequent, tail)
                : alternate()
            const left = returns(statement.consequent, tail)
            const right = alternate()
            return merge(left, right)
          }
          default:
            return null
        }
      }
      return sequence(node.body.body)
    }
    default:
      return null
  }
}
