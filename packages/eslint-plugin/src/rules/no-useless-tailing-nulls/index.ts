import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'
import type { RuleContext } from '@typescript-eslint/utils/ts-eslint'

import { ImportStorage } from '../../utils/import-storage'
import { styleValueRoot } from '../../utils/style-position'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

function checkUselessTailingNulls<T extends RuleContext<string, []>>(
  node: TSESTree.ArrayExpression,
  context: T,
) {
  const lastElement = node.elements[node.elements.length - 1]
  if (
    lastElement?.type !== AST_NODE_TYPES.Literal ||
    lastElement.value !== null
  )
    return
  let trailingCount = 1
  for (let i = node.elements.length - 2; i >= 0; i--) {
    const element = node.elements[i]
    if (
      !element ||
      (element.type === AST_NODE_TYPES.Literal && element.value === null)
    ) {
      trailingCount++
    } else {
      break
    }
  }
  const previousElement =
    node.elements[node.elements.length - trailingCount - 1]
  const firstElement = node.elements[0]
  const removeStart = previousElement
    ? previousElement.range[1]
    : (firstElement?.range[0] ?? node.range[0] + 1)

  context.report({
    node,
    messageId: 'uselessTailingNulls',
    fix(fixer) {
      return fixer.removeRange([removeStart, lastElement.range[1]])
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
    const importStorage = new ImportStorage(context)
    return {
      ImportDeclaration(node) {
        importStorage.addImportByDeclaration(node)
      },
      ArrayExpression(node) {
        if (styleValueRoot(node, importStorage))
          checkUselessTailingNulls(node, context)
      },
    }
  },
})
