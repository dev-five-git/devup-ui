import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'

import { ImportStorage } from '../../utils/import-storage'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

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
      Identifier(node) {
        if (!devupContext) return

        const ancestors = context.sourceCode.getAncestors(devupContext)
        const an = context.sourceCode.getAncestors(node).slice(ancestors.length)
        const properties = an.filter(
          (ancestor) => ancestor.type === AST_NODE_TYPES.Property,
        )
        if (
          !properties.length ||
          properties.some(
            (property) => [...an, node].indexOf(property.key) !== -1,
          )
        )
          return
        const conditionals = an.filter(
          (ancestor) => ancestor.type === AST_NODE_TYPES.ConditionalExpression,
        )
        if (
          conditionals.length &&
          conditionals.some(
            (conditional) => [...an, node].indexOf(conditional.test) !== -1,
          )
        ) {
          return
        }

        context.report({
          node,
          messageId: 'cssUtilsLiteralOnly',
        })
      },
    }
  },
})
