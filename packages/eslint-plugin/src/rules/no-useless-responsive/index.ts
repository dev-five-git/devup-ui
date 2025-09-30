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

function checkUselessResponsive<T extends RuleContext<string, []>>(
  node: TSESTree.ArrayExpression,
  ancestors: TSESTree.Node[],
  context: T,
) {
  if (node.elements.length !== 1) return

  const element = node.elements[0]!
  for (const ancestor of ancestors) {
    switch (ancestor.type) {
      case AST_NODE_TYPES.ConditionalExpression:
        if (ancestors.indexOf(ancestor.test) !== -1) return
        break
      case AST_NODE_TYPES.JSXExpressionContainer:
      case AST_NODE_TYPES.Property:
      case AST_NODE_TYPES.JSXOpeningElement:
      case AST_NODE_TYPES.CallExpression:
      case AST_NODE_TYPES.ObjectExpression:
      case AST_NODE_TYPES.JSXAttribute:
        break
      default:
        return
    }
  }

  context.report({
    node,
    messageId: 'uselessResponsive',
    fix(fixer) {
      return fixer.replaceText(node, context.sourceCode.getText(element!))
    },
  })
}

export const noUselessResponsive = createRule({
  name: 'no-useless-responsive',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      uselessResponsive: 'Responsive are useless. Remove them.',
    },
    type: 'problem',
    fixable: 'code',
    docs: {
      description: 'No useless responsive.',
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
        if (devupContext)
          checkUselessResponsive(
            node,
            context.sourceCode
              .getAncestors(node)
              .slice(context.sourceCode.getAncestors(devupContext).length),
            context,
          )
      },
    }
  },
})
