import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

import { ImportStorage } from '../../utils/import-storage'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

type Scope = TSESLint.Scope.Scope

function findVariable(scope: Scope | null, name: string) {
  for (let current = scope; current; current = current.upper) {
    const variable = current.set.get(name)
    if (variable) return variable
  }
  return undefined
}

/** Built-ins the build runs, which give the same result on every build */
const BUILT_INS = new Set([
  'Array',
  'Boolean',
  'JSON',
  'Math',
  'Number',
  'Object',
  'String',
  'parseFloat',
  'parseInt',
])

function isBuiltIn(node: TSESTree.Node, scope: Scope): boolean {
  return (
    node.type === AST_NODE_TYPES.Identifier &&
    BUILT_INS.has(node.name) &&
    !findVariable(scope, node.name)?.defs.length
  )
}

function isMathRandom(node: TSESTree.MemberExpression, scope: Scope) {
  return (
    isBuiltIn(node.object, scope) &&
    node.object.type === AST_NODE_TYPES.Identifier &&
    node.object.name === 'Math' &&
    node.property.type === AST_NODE_TYPES.Identifier &&
    node.property.name === 'random'
  )
}

/** Whether the build knows `node`'s value: a literal, a constant, or what calls of module functions, imports and built-ins compute from them */
function isStaticValue(
  node: TSESTree.Node,
  scope: Scope,
  seen: Set<string>,
): boolean {
  const all = (nodes: (TSESTree.Node | null)[]) =>
    nodes.every(
      (item) =>
        item !== null &&
        item.type !== AST_NODE_TYPES.SpreadElement &&
        isStaticValue(item, scope, seen),
    )
  switch (node.type) {
    case AST_NODE_TYPES.Literal:
      return true
    case AST_NODE_TYPES.TemplateLiteral:
      return all(node.expressions)
    case AST_NODE_TYPES.BinaryExpression:
      return (
        node.left.type !== AST_NODE_TYPES.PrivateIdentifier &&
        all([node.left, node.right])
      )
    case AST_NODE_TYPES.LogicalExpression:
      return all([node.left, node.right])
    case AST_NODE_TYPES.ConditionalExpression:
      return all([node.test, node.consequent, node.alternate])
    case AST_NODE_TYPES.UnaryExpression:
      return node.operator === '-' && isStaticValue(node.argument, scope, seen)
    case AST_NODE_TYPES.TSAsExpression:
    case AST_NODE_TYPES.TSSatisfiesExpression:
      return isStaticValue(node.expression, scope, seen)
    case AST_NODE_TYPES.ArrayExpression:
      return all(node.elements)
    case AST_NODE_TYPES.ObjectExpression:
      return node.properties.every(
        (property) =>
          property.type === AST_NODE_TYPES.Property &&
          !property.computed &&
          isStaticValue(property.value, scope, seen),
      )
    case AST_NODE_TYPES.MemberExpression:
      return (
        (node.computed
          ? isStaticValue(node.property, scope, seen)
          : !isMathRandom(node, scope)) &&
        (isBuiltIn(node.object, scope) ||
          isStaticValue(node.object, scope, seen))
      )
    case AST_NODE_TYPES.CallExpression:
      return isStaticCallee(node.callee, scope, seen) && all(node.arguments)
    case AST_NODE_TYPES.Identifier:
      return (
        ['undefined', 'NaN', 'Infinity'].includes(node.name) ||
        isStaticBinding(node.name, scope, seen)
      )
    default:
      return false
  }
}

/** Whether the build can run `callee`: a built-in, or a function the module declares or imports */
function isStaticCallee(
  callee: TSESTree.Node,
  scope: Scope,
  seen: Set<string>,
): boolean {
  if (callee.type === AST_NODE_TYPES.MemberExpression) {
    return (
      !callee.computed &&
      !isMathRandom(callee, scope) &&
      (isBuiltIn(callee.object, scope) ||
        isStaticValue(callee.object, scope, seen))
    )
  }
  if (callee.type !== AST_NODE_TYPES.Identifier) return false
  if (isBuiltIn(callee, scope)) return true
  const variable = findVariable(scope, callee.name)
  const definition = variable?.defs[0]
  if (!variable || !definition) return false
  return (
    ['module', 'global'].includes(variable.scope.type) &&
    (definition.type === 'ImportBinding' ||
      definition.type === 'FunctionName' ||
      (definition.type === 'Variable' && definition.parent.kind === 'const'))
  )
}

function isStaticBinding(
  name: string,
  scope: Scope,
  seen: Set<string>,
): boolean {
  if (seen.has(name)) return false
  const variable = findVariable(scope, name)
  const definition = variable?.defs[0]
  if (!variable || !definition) return false
  if (definition.type === 'ImportBinding') return true
  if (
    definition.type !== 'Variable' ||
    definition.parent.kind !== 'const' ||
    !['module', 'global'].includes(variable.scope.type) ||
    definition.node.id.type !== AST_NODE_TYPES.Identifier ||
    !definition.node.init
  )
    return false
  seen.add(name)
  return isStaticValue(definition.node.init, variable.scope, seen)
}

export const cssUtilsLiteralOnly = createRule({
  name: 'css-utils-literal-only',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      cssUtilsLiteralOnly:
        'CSS utils should only be used with values known at build time: literals, constants, or what module functions, imports and built-ins compute from them.',
    },
    type: 'problem',
    docs: {
      description:
        'CSS utils should only be used with values known at build time.',
    },
  },
  create(context) {
    const importStorage = new ImportStorage()
    let devupContext:
      TSESTree.CallExpression | TSESTree.JSXOpeningElement | null = null
    return {
      ImportDeclaration(node) {
        importStorage.addImportByDeclaration(node)
      },
      CallExpression(node) {
        if (
          importStorage.checkContextType(node) === 'UTIL' &&
          node.arguments.length === 1 &&
          node.arguments[0].type === AST_NODE_TYPES.ObjectExpression
        ) {
          devupContext = node
        }
      },
      'CallExpression:exit'(node) {
        if (devupContext === node) {
          devupContext = null
        }
      },
      Identifier(node) {
        if (!devupContext || node.name === 'undefined') return

        const an = context.sourceCode
          .getAncestors(node)
          .slice(context.sourceCode.getAncestors(devupContext).length)
        const scope = context.sourceCode.getScope(node)

        let callee: TSESTree.Node | null = null
        let member: TSESTree.MemberExpression | null = null
        for (const ancestor of an) {
          switch (ancestor.type) {
            case AST_NODE_TYPES.Property:
              if ([...an, node].indexOf(ancestor.key) !== -1) return
              break
            case AST_NODE_TYPES.ConditionalExpression:
              if ([...an, node].indexOf(ancestor.test) !== -1) return
              break
            case AST_NODE_TYPES.MemberExpression:
              if ([...an, node].indexOf(ancestor.property) !== -1) return
              member = ancestor
              break
            case AST_NODE_TYPES.CallExpression:
              if ([...an, node].indexOf(ancestor.callee) !== -1)
                callee = ancestor.callee
              break
          }
        }

        if (
          callee
            ? isStaticCallee(callee, scope, new Set())
            : member
              ? isStaticValue(member, scope, new Set())
              : isStaticBinding(node.name, scope, new Set())
        )
          return

        context.report({
          node,
          messageId: 'cssUtilsLiteralOnly',
        })
      },
    }
  },
})
