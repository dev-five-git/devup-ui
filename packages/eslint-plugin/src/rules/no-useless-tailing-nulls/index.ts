import {
  AST_NODE_TYPES,
  ASTUtils,
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

function isNullLiteral(
  node: TSESTree.Node | null | undefined,
): node is TSESTree.NullLiteral {
  return node?.type === AST_NODE_TYPES.Literal && node.raw === 'null'
}

function checkUselessTailingNulls<T extends RuleContext<string, []>>(
  node: TSESTree.ArrayExpression,
  context: T,
) {
  const lastElement = node.elements[node.elements.length - 1]
  if (!isNullLiteral(lastElement)) return
  let trailingCount = 1
  for (let i = node.elements.length - 2; i >= 0; i--) {
    const element = node.elements[i]
    if (!element || isNullLiteral(element)) {
      trailingCount++
    } else {
      break
    }
  }
  const previousElement =
    node.elements[node.elements.length - trailingCount - 1]
  context.report({
    node,
    messageId: 'uselessTailingNulls',
    fix(fixer) {
      const sourceCode = context.sourceCode
      let removeStart = node.range[0] + 1
      if (previousElement) {
        const [separator] = sourceCode.getFirstTokensBetween(
          previousElement,
          lastElement,
          { count: 1, filter: ASTUtils.isCommaToken },
        )
        const [previousToken] = sourceCode.getTokensBefore(separator, 1)
        removeStart = previousToken.range[1]
      } else if (node.elements[0]) {
        const [, firstToken] = sourceCode.getFirstTokens(node, 2)
        removeStart = firstToken.range[0]
      }
      const [beforeLastToken, lastToken] = sourceCode.getLastTokens(node, 3)
      const removeEnd = ASTUtils.isCommaToken(lastToken)
        ? beforeLastToken.range[1]
        : lastToken.range[1]
      return fixer.removeRange([removeStart, removeEnd])
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
