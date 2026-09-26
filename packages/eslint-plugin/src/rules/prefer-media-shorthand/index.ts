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

const MEDIA_SHORTHANDS = new Map([
  ['all', '_all'],
  ['print', '_print'],
  ['screen', '_screen'],
  ['(prefers-reduced-motion:reduce)', '_motionReduce'],
  ['(prefers-reduced-motion:no-preference)', '_motionSafe'],
  ['(orientation:portrait)', '_portrait'],
  ['(orientation:landscape)', '_landscape'],
  ['(prefers-contrast:more)', '_contrastMore'],
  ['(prefers-contrast:less)', '_contrastLess'],
  ['(forced-colors:active)', '_forcedColors'],
])

const AT_MEDIA = /^@media(?=[\s(])/

function shorthandFor(query: string): string | undefined {
  return MEDIA_SHORTHANDS.get(query.replace(/\s+/g, '').toLowerCase())
}

export const preferMediaShorthand = createRule({
  name: 'prefer-media-shorthand',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      preferMediaShorthand: 'Use the `{{shorthand}}` shorthand for this query.',
    },
    type: 'suggestion',
    fixable: 'code',
    docs: {
      description: 'Prefer media shorthand props over their media queries.',
    },
  },
  create(context) {
    const importStorage = new ImportStorage()
    let devupContext:
      TSESTree.CallExpression | TSESTree.JSXOpeningElement | null = null

    function checkMediaRecord(
      owner: TSESTree.Property | TSESTree.JSXAttribute,
      record: TSESTree.Node,
    ) {
      if (record.type !== AST_NODE_TYPES.ObjectExpression) return
      for (const prop of record.properties) {
        if (prop.type !== AST_NODE_TYPES.Property) continue
        const query = propertyKeyName(prop)
        const shorthand = query === undefined ? undefined : shorthandFor(query)
        if (!shorthand) continue
        const value = context.sourceCode.getText(prop.value)
        context.report({
          node: prop.key,
          messageId: 'preferMediaShorthand',
          data: { shorthand },
          fix:
            record.properties.length === 1
              ? (fixer) =>
                  fixer.replaceText(
                    owner,
                    owner.type === AST_NODE_TYPES.JSXAttribute
                      ? `${shorthand}={${value}}`
                      : `${shorthand}: ${value}`,
                  )
              : null,
        })
      }
    }

    return {
      ImportDeclaration(node) {
        importStorage.addImportByDeclaration(node)
      },
      CallExpression(node) {
        if (
          !devupContext &&
          importStorage.checkContextType(node) === 'UTIL' &&
          node.arguments.length === 1 &&
          node.arguments[0].type === AST_NODE_TYPES.ObjectExpression
        ) {
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
      JSXAttribute(node) {
        if (
          devupContext &&
          node.name.type === AST_NODE_TYPES.JSXIdentifier &&
          node.name.name === '_media' &&
          node.value?.type === AST_NODE_TYPES.JSXExpressionContainer
        ) {
          checkMediaRecord(node, node.value.expression)
        }
      },
      Property(node) {
        if (!devupContext) return
        const name = propertyKeyName(node)
        if (name === undefined) return
        if (name === '_media') {
          checkMediaRecord(node, node.value)
          return
        }
        const shorthand = AT_MEDIA.test(name)
          ? shorthandFor(name.slice('@media'.length))
          : undefined
        if (shorthand) {
          context.report({
            node: node.key,
            messageId: 'preferMediaShorthand',
            data: { shorthand },
            fix: node.computed
              ? null
              : (fixer) => fixer.replaceText(node.key, shorthand),
          })
        }
      },
    }
  },
})
