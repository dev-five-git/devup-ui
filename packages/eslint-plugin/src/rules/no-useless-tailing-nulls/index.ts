import { ESLintUtils } from '@typescript-eslint/utils'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

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
  create(_context) {
    return {}
  },
})
