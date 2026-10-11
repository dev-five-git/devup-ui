import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import {
  imported,
  keyOf,
  type Mode,
  modeOf,
  styledFactory,
} from './api-context'
import { createLiteralTree, type TextDiagnostic } from './literal-tree'
import { constant, type ScopeOf, staticValue, unwrap } from './static-value'

export type Site = {
  readonly mode: Mode
  readonly api: string
  readonly namespaceMode?: Mode
  readonly returnsRules?: boolean
  readonly conditional?: boolean
  readonly record?: boolean
  readonly text?: boolean
  readonly allowText?: boolean
  readonly declarations?: boolean
  readonly callbacks?: boolean
}
type OrderCheck = (input: TSESTree.Node, site: Site) => void

const DATA = new Set(['vars', 'imports', 'params', 'props', 'styleVars'])
const RECORDS = new Set([
  'selectors',
  '@layer',
  '_media',
  '@media',
  '_supports',
  '@supports',
  '_container',
  '@container',
])

export function createStyleTree(
  scopeOf: ScopeOf,
  check: OrderCheck,
  report: (diagnostic: TextDiagnostic) => void,
) {
  const literal = createLiteralTree(scopeOf, {
    check,
    report,
    walk: (node, site) => walk(node, site),
  })
  const walk = (
    input: TSESTree.Node,
    site: Site,
    seen = new Set<TSESTree.Node>(),
  ) => {
    const node = unwrap(input)
    if (seen.has(node)) return
    const next = new Set(seen).add(node)
    const { mode, api, namespaceMode = 'stylex', returnsRules = false } = site
    switch (node.type) {
      case AST_NODE_TYPES.Identifier: {
        const init = constant(node, scopeOf)
        if (init) walk(init, site, next)
        break
      }
      case AST_NODE_TYPES.Literal:
      case AST_NODE_TYPES.TemplateLiteral:
        literal(node, site)
        break
      case AST_NODE_TYPES.TaggedTemplateExpression:
        if (
          modeOf(imported(node.tag, scopeOf)) ||
          styledFactory(node.tag, scopeOf)
        )
          walk(node.quasi, site, next)
        break
      case AST_NODE_TYPES.ObjectExpression:
        for (const property of node.properties) {
          if (property.type === AST_NODE_TYPES.SpreadElement) {
            walk(property.argument, site, next)
            continue
          }
          const key = keyOf(property, scopeOf)
          if (site.record) {
            walk(
              property.value,
              { ...site, record: false, text: true, declarations: true },
              next,
            )
          } else if (mode === 'namespaces') {
            walk(
              property.value,
              { mode: namespaceMode, api, returnsRules: true },
              next,
            )
          } else if (key === 'styleOrder' || key === 'style-order') {
            check(property.value, site)
          } else if (key === 'fontFaces' && mode === 'global') {
            walk(
              property.value,
              { mode: 'fontface', api: `${api}.fontFaces` },
              next,
            )
          } else if (key === 'fontFaces') {
            continue
          } else if (key !== null && !DATA.has(key)) {
            walk(
              property.value,
              {
                ...site,
                mode: /^@font-face\b/i.test(key)
                  ? 'fontface'
                  : /^@(?:[\w-]*keyframes)\b/i.test(key)
                    ? 'keyframes'
                    : mode,
                returnsRules: false,
                record: RECORDS.has(key),
                declarations: !/^@(?:[\w-]*keyframes)\b/i.test(key),
                text:
                  RECORDS.has(key) ||
                  key.startsWith('_') ||
                  key.startsWith('@') ||
                  key.startsWith('&') ||
                  key.startsWith(':') ||
                  ((mode === 'global' || mode === 'keyframes') &&
                    !site.declarations),
              },
              next,
            )
          }
        }
        break
      case AST_NODE_TYPES.ArrayExpression:
        for (const element of node.elements) {
          if (element)
            walk(
              element.type === AST_NODE_TYPES.SpreadElement
                ? element.argument
                : element,
              site,
              next,
            )
        }
        break
      case AST_NODE_TYPES.ConditionalExpression: {
        const test = staticValue(node.test, scopeOf)
        if (test)
          walk(test.value ? node.consequent : node.alternate, site, next)
        else {
          const branch = {
            ...site,
            conditional: mode === 'global' || site.conditional,
          }
          walk(node.consequent, branch, next)
          walk(node.alternate, branch, next)
        }
        break
      }
      case AST_NODE_TYPES.LogicalExpression: {
        const left = staticValue(node.left, scopeOf)
        if (left) {
          const right =
            node.operator === '&&'
              ? Boolean(left.value)
              : node.operator === '||'
                ? !left.value
                : left.value === null || left.value === undefined
          walk(right ? node.right : node.left, site, next)
        } else {
          const branch = {
            ...site,
            conditional: mode === 'global' || site.conditional,
          }
          walk(node.left, branch, next)
          walk(node.right, branch, next)
        }
        break
      }
      case AST_NODE_TYPES.ArrowFunctionExpression:
      case AST_NODE_TYPES.FunctionExpression:
        if (returnsRules) {
          if (node.body.type === AST_NODE_TYPES.BlockStatement) {
            for (const statement of node.body.body) {
              if (
                statement.type === AST_NODE_TYPES.ReturnStatement &&
                statement.argument
              )
                walk(statement.argument, { ...site, returnsRules: false }, next)
            }
          } else walk(node.body, { ...site, returnsRules: false }, next)
        }
        break
    }
  }

  const props = (
    input: TSESTree.Node,
    site: {
      readonly devup: boolean
      readonly takesCss: boolean
      readonly api: string
      readonly global?: boolean
    },
  ) => {
    const node = unwrap(input)
    if (node.type === AST_NODE_TYPES.Identifier) {
      const init = constant(node, scopeOf)
      if (
        init &&
        init !== node &&
        unwrap(init).type === AST_NODE_TYPES.ObjectExpression
      )
        props(init, site)
      return
    }
    if (node.type !== AST_NODE_TYPES.ObjectExpression) return
    const { devup, takesCss, api } = site
    for (const property of node.properties) {
      if (property.type === AST_NODE_TYPES.SpreadElement) {
        if (devup || takesCss || site.global) props(property.argument, site)
        continue
      }
      const key = keyOf(property, scopeOf)
      if (key === 'styles' && site.global)
        walk(property.value, { mode: 'global', api })
      else if (key === 'css' && takesCss)
        walk(property.value, { mode: 'class', api, returnsRules: true })
      else if (devup && key === 'styleOrder')
        check(property.value, { mode: 'class', api })
      else if (
        devup &&
        key &&
        !DATA.has(key) &&
        (key.startsWith('_') || key.startsWith('@') || key === 'selectors')
      )
        walk(property.value, { mode: 'class', api })
    }
  }
  return { walk, props }
}
