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

function checkDuplicateValue<T extends RuleContext<string, []>>(
  node: TSESTree.ArrayExpression,
  context: T,
) {
  for (let i = 0; i < node.elements.length; i++) {
    const element = node.elements[i]
    if (element?.type === AST_NODE_TYPES.Literal) {
      if (i === 0) continue
      const prevElement = node.elements[i - 1]
      if (
        prevElement?.type === AST_NODE_TYPES.Literal &&
        element.value === prevElement.value
      ) {
        context.report({
          node,
          messageId: 'duplicateValue',
          data: {
            value: element.value,
          },
          fix(fixer) {
            return fixer.replaceText(element, 'null')
          },
        })
      }
    }
  }
}

export const noDuplicateValue = createRule({
  name: 'no-duplicate-value',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      duplicateValue: 'Duplicate value found: {{value}}.',
    },
    type: 'problem',
    fixable: 'code',
    docs: {
      description: 'No duplicate value.',
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
      ArrayExpression(node) {
        if (devupContext) checkDuplicateValue(node, context)
      },
    }
  },
})
