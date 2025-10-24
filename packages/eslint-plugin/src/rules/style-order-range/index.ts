import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'
import type { RuleContext } from '@typescript-eslint/utils/ts-eslint'

import { ImportStorage } from '../../utils/import-storage'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

function checkStyleOrderRange<T extends RuleContext<string, any[]>>(
  expression: TSESTree.Expression,
  context: T,
) {
  let value: number | null = null

  if (expression.type === AST_NODE_TYPES.Literal) {
    if (typeof expression.value === 'number') {
      value = expression.value
    } else if (typeof expression.value === 'string') {
      const parsed = parseInt(expression.value, 10)
      if (!isNaN(parsed)) {
        value = parsed
      }
    }
  } else if (expression.type === AST_NODE_TYPES.UnaryExpression) {
    if (
      expression.argument.type === AST_NODE_TYPES.Literal &&
      typeof expression.argument.value === 'number' &&
      (expression.operator === '-' || expression.operator === '+')
    ) {
      value =
        expression.operator === '-'
          ? -expression.argument.value
          : expression.argument.value
    } else {
      context.report({
        node: expression,
        messageId: 'styleOrderRange',
      })
      return
    }
  } else if (expression.type === AST_NODE_TYPES.TemplateLiteral) {
    if (expression.expressions.length > 0) {
      // error report
      context.report({
        node: expression,
        messageId: 'styleOrderRange',
      })
      return
    } else {
      value = parseInt(expression.quasis[0].value.raw, 10)
      if (isNaN(value)) {
        // error report
        context.report({
          node: expression,
          messageId: 'styleOrderRange',
        })
        return
      }
    }
  }

  if (value === null || value < 1 || value > 254) {
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
        'styleOrder prop must be a number greater than 0 and less than 255.',
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
    const importStorage = new ImportStorage()
    let devupContext:
      | TSESTree.CallExpression
      | TSESTree.JSXOpeningElement
      | null = null

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
      Property(node) {
        if (
          devupContext &&
          node.key.type === AST_NODE_TYPES.Identifier &&
          node.key.name === 'styleOrder' &&
          node.value.type !== AST_NODE_TYPES.AssignmentPattern &&
          node.value.type !== AST_NODE_TYPES.TSEmptyBodyFunctionExpression
        ) {
          checkStyleOrderRange(node.value, context)
        }
      },
      JSXOpeningElement(node) {
        if (importStorage.checkContextType(node) === 'COMPONENT') {
          devupContext = node
        }
      },
      'JSXOpeningElement:exit'(node) {
        if (devupContext === node) {
          devupContext = null
        }
      },
      JSXAttribute(node) {
        if (!devupContext) return
        // styleOrder prop만 체크
        if (
          node.name.type !== AST_NODE_TYPES.JSXIdentifier ||
          node.name.name !== 'styleOrder' ||
          !node.value
        ) {
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
