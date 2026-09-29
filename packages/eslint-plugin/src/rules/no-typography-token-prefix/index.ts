import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'

import { ImportStorage } from '../../utils/import-storage'
import { propertyKeyName } from '../../utils/property-key-name'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

/** Whether `literal` is a value of a `typography` prop rather than a condition. */
function isTypographyValue(
  child: TSESTree.Node,
  parent: TSESTree.Node | undefined = child.parent,
): boolean {
  switch (parent?.type) {
    case AST_NODE_TYPES.JSXAttribute:
      return (
        parent.name.type === AST_NODE_TYPES.JSXIdentifier &&
        parent.name.name === 'typography'
      )
    case AST_NODE_TYPES.Property:
      return parent.value === child && propertyKeyName(parent) === 'typography'
    case AST_NODE_TYPES.ConditionalExpression:
      return parent.test !== child && isTypographyValue(parent)
    case AST_NODE_TYPES.JSXExpressionContainer:
    case AST_NODE_TYPES.ArrayExpression:
    case AST_NODE_TYPES.LogicalExpression:
      return isTypographyValue(parent)
    default:
      return false
  }
}

export const noTypographyTokenPrefix = createRule({
  name: 'no-typography-token-prefix',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      noTypographyTokenPrefix:
        'Typography keys take no `$` prefix. Use "{{name}}".',
    },
    type: 'problem',
    fixable: 'code',
    docs: {
      description: 'Disallow the `$` token prefix on typography keys.',
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
        if (!devupContext && importStorage.checkContextType(node) === 'UTIL') {
          devupContext = node
        }
      },
      'CallExpression:exit'(node) {
        if (devupContext === node) devupContext = null
      },
      JSXOpeningElement(node) {
        if (importStorage.checkContextType(node) === 'COMPONENT') {
          devupContext = node
        }
      },
      'JSXOpeningElement:exit'(node) {
        if (devupContext === node) devupContext = null
      },
      Literal(node) {
        if (
          !devupContext ||
          typeof node.value !== 'string' ||
          !node.value.startsWith('$') ||
          !isTypographyValue(node)
        )
          return
        const name = node.value.slice(1)
        const quote = node.raw[0]
        context.report({
          node,
          messageId: 'noTypographyTokenPrefix',
          data: { name },
          fix: (fixer) => fixer.replaceText(node, `${quote}${name}${quote}`),
        })
      },
    }
  },
})
