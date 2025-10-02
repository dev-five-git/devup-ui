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

function checkUselessTailingNulls<T extends RuleContext<string, []>>(
  node: TSESTree.ArrayExpression,
  context: T,
) {
  let nullCount = 0
  for (let i = node.elements.length - 1; i >= 0; i--) {
    const element = node.elements[i]
    if (element?.type === AST_NODE_TYPES.Literal && element.value === null) {
      nullCount++
    } else {
      break
    }
  }
  if (nullCount === 0) return
  context.report({
    node,
    messageId: 'uselessTailingNulls',
    fix(fixer) {
      return fixer.removeRange([
        node.elements.length > nullCount
          ? node.elements[node.elements.length - nullCount - 1]!.range[1]
          : node.elements[0]!.range[0],

        node.elements[node.elements.length - 1]!.range[1],
      ])
    },
  })
}

export const noUselessTailingNulls = createRule({
  name: 'no-useless-tailing-nulls',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      uselessTailingNulls: 'Trailing nulls are useless. Remove them.',
    },
    type: 'problem',
    fixable: 'code',
    docs: {
      description: 'No useless tailing nulls.',
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
        if (
          devupContext &&
          node.parent?.type !== AST_NODE_TYPES.MemberExpression
        )
          checkUselessTailingNulls(node, context)
      },
    }
  },
})
