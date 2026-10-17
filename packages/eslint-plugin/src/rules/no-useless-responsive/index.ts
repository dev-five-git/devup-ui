import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'
import type { RuleContext } from '@typescript-eslint/utils/ts-eslint'

import { ImportStorage } from '../../utils/import-storage'
import { styleValueSite } from '../../utils/style-position'

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

  const element = node.elements[0]
  if (!element) return
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
      return fixer.replaceText(node, context.sourceCode.getText(element))
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
    const importStorage = new ImportStorage(context)
    return {
      ImportDeclaration(node) {
        importStorage.addImportByDeclaration(node)
      },
      ArrayExpression(node) {
        const site = styleValueSite(node, importStorage)
        if (site)
          checkUselessResponsive(
            node,
            context.sourceCode
              .getAncestors(node)
              .slice(context.sourceCode.getAncestors(site.start).length),
            context,
          )
      },
    }
  },
})
