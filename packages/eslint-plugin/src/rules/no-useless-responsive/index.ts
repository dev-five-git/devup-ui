import { ESLintUtils } from '@typescript-eslint/utils'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

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
  create(_context) {
    return {}
  },
})
