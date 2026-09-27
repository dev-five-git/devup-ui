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

/** Whether the build knows `node`'s value: a literal, or what an imported or module-level `const` binds */
function isStaticValue(
  node: TSESTree.Node,
  scope: Scope,
  seen: Set<string>,
): boolean {
  switch (node.type) {
    case AST_NODE_TYPES.Literal:
      return true
    case AST_NODE_TYPES.TemplateLiteral:
      return node.expressions.every((e) => isStaticValue(e, scope, seen))
    case AST_NODE_TYPES.BinaryExpression:
      return (
        node.left.type !== AST_NODE_TYPES.PrivateIdentifier &&
        isStaticValue(node.left, scope, seen) &&
        isStaticValue(node.right, scope, seen)
      )
    case AST_NODE_TYPES.UnaryExpression:
      return node.operator === '-' && isStaticValue(node.argument, scope, seen)
    case AST_NODE_TYPES.TSAsExpression:
    case AST_NODE_TYPES.TSSatisfiesExpression:
      return isStaticValue(node.expression, scope, seen)
    case AST_NODE_TYPES.ObjectExpression:
      return node.properties.every(
        (property) =>
          property.type === AST_NODE_TYPES.Property &&
          !property.computed &&
          isStaticValue(property.value, scope, seen),
      )
    case AST_NODE_TYPES.MemberExpression:
      return !node.computed && isStaticValue(node.object, scope, seen)
    case AST_NODE_TYPES.Identifier:
      return isStaticBinding(node.name, scope, seen)
    default:
      return false
  }
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
      cssUtilsLiteralOnly: 'CSS utils should only be used with literal values.',
    },
    type: 'problem',
    docs: {
      description: 'CSS utils should only be used with literal values.',
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
              break
            case AST_NODE_TYPES.CallExpression:
              if ([...an, node].indexOf(ancestor.callee) !== -1) return
              break
          }
        }

        if (
          isStaticBinding(
            node.name,
            context.sourceCode.getScope(node),
            new Set(),
          )
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
