import { ESLintUtils } from '@typescript-eslint/utils'

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
    fixable: 'code',
    docs: {
      description: 'CSS utils should only be used with literal values.',
    },
  },
  create(_context) {
    return {}
  },
})
