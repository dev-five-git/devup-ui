import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'

import {
  argumentsOf,
  component,
  imported,
  jsxApi,
  jsxName,
  keyOf,
  modeOf,
  styleComponent,
  styledFactory,
} from './api-context'
import { deferredValue } from './deferred-value'
import { staticValue, unwrap, validOrder } from './static-value'
import { createStyleTree, type Site } from './style-tree'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

export const styleOrderRange = createRule({
  name: 'style-order-range',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      styleOrderRange:
        'styleOrder must be an integer Number or a canonical decimal string from 1 through 254, known at build time; class styles may choose valid orders with a conditional or &&.',
      wrongType: 'styleOrder must have an explicit value.',
      unsupportedOrder:
        '{{api}} cannot use styleOrder: order metadata has no effect in keyframes, font-face descriptors or native StyleX declarations.',
      globalOrder:
        '{{api}} cannot use conditional styleOrder: global styles require one order known at build time.',
    },
    type: 'problem',
    docs: {
      description:
        'Enforces build-time styleOrder values and supported API contexts.',
    },
  },
  create(context) {
    const scopeOf = (node: TSESTree.Node) => context.sourceCode.getScope(node)
    const reported = new Set<TSESTree.Node>()
    let emotionCss = false
    const check = (input: TSESTree.Node, site: Site) => {
      if (reported.has(input)) return
      const { mode, api } = site
      const node = unwrap(input)
      const value = staticValue(node, scopeOf)
      if (mode === 'keyframes' || mode === 'stylex' || mode === 'fontface') {
        reported.add(input)
        context.report({
          node: input,
          messageId: 'unsupportedOrder',
          data: { api },
        })
        return
      }
      const branches = (expression: TSESTree.Node): boolean => {
        const branch = unwrap(expression)
        const result = staticValue(branch, scopeOf)
        if (result) return validOrder(result.value)
        if (branch.type === AST_NODE_TYPES.ConditionalExpression)
          return branches(branch.consequent) && branches(branch.alternate)
        if (branch.type === AST_NODE_TYPES.LogicalExpression)
          return (
            (branch.operator === '&&' && branches(branch.right)) ||
            deferredValue(branch, scopeOf)
          )
        return deferredValue(branch, scopeOf)
      }
      if (
        !site.conditional &&
        (value
          ? validOrder(value.value)
          : mode === 'class'
            ? branches(node)
            : deferredValue(node, scopeOf))
      )
        return
      reported.add(input)
      context.report({
        node: input,
        messageId:
          mode === 'global' &&
          (site.conditional ||
            (!value &&
              (node.type === AST_NODE_TYPES.ConditionalExpression ||
                node.type === AST_NODE_TYPES.LogicalExpression)))
            ? 'globalOrder'
            : 'styleOrderRange',
        data: { api },
      })
    }
    const { walk, props } = createStyleTree(scopeOf, check)
    return {
      ImportDeclaration(node) {
        if (node.source.value === '@emotion/react') emotionCss = true
      },
      CallExpression(node) {
        const args = argumentsOf(node.arguments, scopeOf)
        const api = imported(node.callee, scopeOf)
        const mode = modeOf(api)
        if (mode) {
          const label = `${api?.source}.${api?.name}`
          for (const argument of args) walk(argument, { mode, api: label })
        } else if (
          api?.source === '@vanilla-extract/css' &&
          api.name === 'styleVariants'
        ) {
          if (args[0])
            walk(args[0], {
              mode: 'namespaces',
              api: 'vanilla-extract.styleVariants',
              namespaceMode: 'class',
            })
          if (args[1])
            walk(args[1], {
              mode: 'class',
              api: 'vanilla-extract.styleVariants',
              returnsRules: true,
            })
        } else if (jsxApi(api) && args.length >= 2) {
          const devup = styleComponent(args[0], scopeOf)
          const owner = imported(args[0], scopeOf)
          props(args[1], {
            devup,
            api: api?.name ?? 'jsx',
            takesCss:
              devup || api?.source.startsWith('@emotion/react') === true,
            global:
              owner?.name === 'Global' &&
              ['@emotion/react', '@devup-ui/react/compat'].includes(
                owner.source,
              ),
          })
        } else if (styledFactory(node.callee, scopeOf)) {
          const callee = unwrap(node.callee)
          if (
            callee.type === AST_NODE_TYPES.MemberExpression &&
            ['attrs', 'withConfig'].includes(keyOf(callee, scopeOf) ?? '')
          )
            return
          const start = imported(callee, scopeOf)?.name === 'styled' ? 1 : 0
          for (const argument of args.slice(start))
            walk(argument, { mode: 'class', api: 'styled', returnsRules: true })
        }
      },
      JSXOpeningElement(node) {
        const api = jsxName(node.name, scopeOf)
        const devup =
          component(api) ||
          (node.name.type === AST_NODE_TYPES.JSXIdentifier &&
            styleComponent(node.name, scopeOf))
        const global =
          api?.name === 'Global' &&
          ['@emotion/react', '@devup-ui/react/compat'].includes(api.source)
        for (const attribute of node.attributes) {
          if (attribute.type === AST_NODE_TYPES.JSXSpreadAttribute) {
            if (devup || global)
              props(attribute.argument, {
                devup,
                takesCss: devup,
                global,
                api: 'JSX',
              })
            continue
          }
          if (attribute.name.type !== AST_NODE_TYPES.JSXIdentifier) continue
          const key = attribute.name.name
          const value =
            attribute.value?.type === AST_NODE_TYPES.JSXExpressionContainer
              ? attribute.value.expression
              : attribute.value
          if (devup && key === 'styleOrder') {
            if (
              value?.type === AST_NODE_TYPES.JSXElement ||
              value?.type === AST_NODE_TYPES.JSXFragment
            )
              context.report({ node: attribute, messageId: 'wrongType' })
            else if (value) check(value, { mode: 'class', api: 'JSX' })
            else context.report({ node: attribute, messageId: 'wrongType' })
          } else if (value && global && key === 'styles')
            walk(value, { mode: 'global', api: 'Global' })
          else if (
            value &&
            key === 'css' &&
            (devup ||
              (emotionCss &&
                node.name.type === AST_NODE_TYPES.JSXIdentifier &&
                /^[a-z]/.test(node.name.name)))
          )
            walk(value, { mode: 'class', api: 'css prop', returnsRules: true })
          else if (
            value &&
            devup &&
            (key.startsWith('_') || key.startsWith('@') || key === 'selectors')
          )
            walk(value, { mode: 'class', api: 'JSX' })
        }
      },
    }
  },
})
