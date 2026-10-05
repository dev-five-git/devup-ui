import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'
import type { RuleContext } from '@typescript-eslint/utils/ts-eslint'

import { ImportStorage, STYLE_COMPONENTS } from '../../utils/import-storage'
import { componentName, styleArguments } from '../../utils/style-position'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

function isStyleOrder(value: number | string): boolean {
  const number = Number(value)
  return (
    Number.isInteger(number) &&
    number >= 1 &&
    number <= 254 &&
    (typeof value === 'number' || value === String(number))
  )
}

function numericValue(expression: TSESTree.Expression): number | null {
  switch (expression.type) {
    case AST_NODE_TYPES.Literal:
      return typeof expression.value === 'number'
        ? expression.value
        : typeof expression.value === 'string' ||
            typeof expression.value === 'boolean' ||
            expression.value === null
          ? Number(expression.value)
          : null
    case AST_NODE_TYPES.TemplateLiteral:
      return expression.expressions.length === 0 &&
        expression.quasis[0].value.cooked !== null
        ? Number(expression.quasis[0].value.cooked)
        : null
    case AST_NODE_TYPES.UnaryExpression: {
      if (expression.operator !== '+' && expression.operator !== '-')
        return null
      const value = numericValue(expression.argument)
      return value === null
        ? null
        : expression.operator === '-'
          ? -value
          : value
    }
    case AST_NODE_TYPES.TSAsExpression:
    case AST_NODE_TYPES.TSSatisfiesExpression:
    case AST_NODE_TYPES.TSNonNullExpression:
    case AST_NODE_TYPES.TSTypeAssertion:
      return numericValue(expression.expression)
    default:
      return null
  }
}

function isStyleOrderExpression(expression: TSESTree.Expression): boolean {
  switch (expression.type) {
    case AST_NODE_TYPES.Literal:
      return (
        (typeof expression.value === 'number' ||
          typeof expression.value === 'string') &&
        isStyleOrder(expression.value)
      )
    case AST_NODE_TYPES.UnaryExpression: {
      const value = numericValue(expression)
      return value !== null && isStyleOrder(value)
    }
    case AST_NODE_TYPES.TemplateLiteral:
      return (
        expression.expressions.length === 0 &&
        expression.quasis[0].value.cooked !== null &&
        isStyleOrder(expression.quasis[0].value.cooked)
      )
    case AST_NODE_TYPES.ConditionalExpression:
      return (
        isStyleOrderExpression(expression.consequent) &&
        isStyleOrderExpression(expression.alternate)
      )
    case AST_NODE_TYPES.LogicalExpression:
      return (
        expression.operator === '&&' && isStyleOrderExpression(expression.right)
      )
    case AST_NODE_TYPES.TSAsExpression:
    case AST_NODE_TYPES.TSSatisfiesExpression:
    case AST_NODE_TYPES.TSNonNullExpression:
    case AST_NODE_TYPES.TSTypeAssertion:
      return isStyleOrderExpression(expression.expression)
    default:
      return false
  }
}

function checkStyleOrderRange<T extends RuleContext<string, []>>(
  expression: TSESTree.Expression,
  context: T,
) {
  if (!isStyleOrderExpression(expression)) {
    context.report({
      node: expression,
      messageId: 'styleOrderRange',
    })
  }
}

export const styleOrderRange = createRule({
  name: 'style-order-range',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      styleOrderRange:
        'styleOrder must be an integer from 1 to 254 or its decimal digit string, with no signs, spaces or leading zeros.',
      wrongType:
        'styleOrder prop must be a number or a string representing a number.',
    },
    type: 'problem',
    docs: {
      description:
        'Ensures styleOrder prop is within valid range (0 < value < 255).',
    },
  },
  create(context) {
    const importStorage = new ImportStorage(context)

    return {
      ImportDeclaration(node) {
        importStorage.addImportByDeclaration(node)
      },
      Property(node) {
        // The build reads `styleOrder` only as a key of a style object handed to the utility itself
        const object = node.parent
        const call = object.parent
        if (
          !node.computed &&
          ((node.key.type === AST_NODE_TYPES.Identifier &&
            node.key.name === 'styleOrder') ||
            (node.key.type === AST_NODE_TYPES.Literal &&
              node.key.value === 'styleOrder')) &&
          node.value.type !== AST_NODE_TYPES.AssignmentPattern &&
          node.value.type !== AST_NODE_TYPES.TSEmptyBodyFunctionExpression &&
          object.type === AST_NODE_TYPES.ObjectExpression &&
          call?.type === AST_NODE_TYPES.CallExpression &&
          call.arguments.includes(object) &&
          styleArguments(call, importStorage)?.includes(object)
        ) {
          checkStyleOrderRange(node.value, context)
        }
      },
      JSXAttribute(node) {
        if (
          node.name.type !== AST_NODE_TYPES.JSXIdentifier ||
          node.name.name !== 'styleOrder' ||
          !STYLE_COMPONENTS.has(
            componentName(node.parent.name, importStorage) ?? '',
          )
        ) {
          return
        }

        if (!node.value) {
          context.report({ node, messageId: 'styleOrderRange' })
          return
        }

        if (
          node.value.type === AST_NODE_TYPES.JSXExpressionContainer &&
          node.value.expression.type !== AST_NODE_TYPES.JSXEmptyExpression
        ) {
          checkStyleOrderRange(node.value.expression, context)
        } else if (node.value.type === AST_NODE_TYPES.Literal) {
          checkStyleOrderRange(node.value, context)
        } else {
          context.report({
            node: node,
            messageId: 'wrongType',
          })
        }
      },
    }
  },
})
