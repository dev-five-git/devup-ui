import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import {
  constant,
  type ScopeOf,
  staticValue,
  unwrap,
  variableOf,
} from './static-value'

export type Api = { readonly source: string; readonly name: string }
export type Mode =
  'class' | 'global' | 'keyframes' | 'stylex' | 'namespaces' | 'fontface'

const DEVUP = '@devup-ui/react'
const LIBRARIES = new Set([
  DEVUP,
  `${DEVUP}/compat`,
  '@emotion/react',
  '@emotion/css',
  '@emotion/styled',
  'styled-components',
  '@vanilla-extract/css',
  '@stylexjs/stylex',
  'react',
  'react/jsx-runtime',
  'react/jsx-dev-runtime',
  '@emotion/react/jsx-runtime',
  '@emotion/react/jsx-dev-runtime',
])
const COMPONENTS = new Set([
  'Box',
  'Button',
  'Center',
  'Flex',
  'Grid',
  'Image',
  'Input',
  'Text',
  'VStack',
])

export function keyOf(
  node: TSESTree.Property | TSESTree.MemberExpression,
  scopeOf: ScopeOf,
): string | null {
  const key = node.type === AST_NODE_TYPES.Property ? node.key : node.property
  if (!node.computed && key.type === AST_NODE_TYPES.Identifier) return key.name
  const value = staticValue(key, scopeOf)
  return value &&
    (typeof value.value === 'string' || typeof value.value === 'number')
    ? String(value.value)
    : null
}

/** Resolve the actual lexical import, not a same-named local or callback parameter. */
export function imported(
  input: TSESTree.Node,
  scopeOf: ScopeOf,
  seen = new Set<TSESTree.Node>(),
): Api | null {
  const node = unwrap(input)
  if (seen.has(node)) return null
  const next = new Set(seen).add(node)
  if (node.type === AST_NODE_TYPES.MemberExpression) {
    const root = imported(node.object, scopeOf, next)
    const name = keyOf(node, scopeOf)
    return root?.name === '*' && name ? { source: root.source, name } : null
  }
  if (
    node.type !== AST_NODE_TYPES.Identifier &&
    node.type !== AST_NODE_TYPES.JSXIdentifier
  )
    return null
  const definition = variableOf(node, scopeOf)?.defs[0]
  if (definition?.type === 'Parameter') {
    const callback = definition.node
    const parameter = callback.params[0]
    const container = callback.parent
    if (
      parameter?.type === AST_NODE_TYPES.ObjectPattern &&
      container.type === AST_NODE_TYPES.JSXExpressionContainer &&
      container.parent.type === AST_NODE_TYPES.JSXElement
    ) {
      const owner = jsxName(container.parent.openingElement.name, scopeOf)
      if (
        owner?.name === 'ClassNames' &&
        ['@emotion/react', `${DEVUP}/compat`].includes(owner.source) &&
        parameter.properties.some(
          (property) =>
            property.type === AST_NODE_TYPES.Property &&
            keyOf(property, scopeOf) === 'css' &&
            property.value === definition.name,
        )
      )
        return { source: owner.source, name: 'css' }
    }
  }
  if (
    definition?.type === 'ImportBinding' &&
    definition.parent.type === AST_NODE_TYPES.ImportDeclaration
  ) {
    const source = definition.parent.source.value
    if (!LIBRARIES.has(source)) return null
    const specifier = definition.node
    switch (specifier.type) {
      case AST_NODE_TYPES.ImportSpecifier:
        return {
          source,
          name:
            specifier.imported.type === AST_NODE_TYPES.Identifier
              ? specifier.imported.name
              : specifier.imported.value,
        }
      case AST_NODE_TYPES.ImportDefaultSpecifier:
        return {
          source,
          name:
            source === 'styled-components' || source === '@emotion/styled'
              ? 'styled'
              : '*',
        }
      case AST_NODE_TYPES.ImportNamespaceSpecifier:
        return { source, name: '*' }
    }
  }
  const init = constant(node, scopeOf)
  return init ? imported(init, scopeOf, next) : null
}

export function component(api: Api | null): boolean {
  return api !== null && api.source === DEVUP && COMPONENTS.has(api.name)
}

export function modeOf(api: Api | null): Mode | null {
  if (!api) return null
  if (api.source === '@stylexjs/stylex') {
    switch (api.name) {
      case 'create':
        return 'namespaces'
      case 'keyframes':
        return 'keyframes'
      case 'positionTry':
      case 'viewTransitionClass':
        return 'stylex'
      default:
        return null
    }
  }
  if (
    ['react', 'react/jsx-runtime', 'react/jsx-dev-runtime'].includes(api.source)
  )
    return null
  switch (api.name) {
    case 'css':
      return 'class'
    case 'style':
      return api.source === '@vanilla-extract/css' ? 'class' : null
    case 'globalCss':
    case 'globalStyle':
    case 'createGlobalStyle':
      return 'global'
    case 'keyframes':
      return 'keyframes'
    default:
      return null
  }
}

export function styledFactory(input: TSESTree.Node, scopeOf: ScopeOf): boolean {
  const node = unwrap(input)
  const api = imported(node, scopeOf)
  if (api?.name === 'styled') return true
  switch (node.type) {
    case AST_NODE_TYPES.MemberExpression:
      return styledFactory(node.object, scopeOf)
    case AST_NODE_TYPES.CallExpression:
      return styledFactory(node.callee, scopeOf)
    default:
      return false
  }
}

export function jsxApi(api: Api | null): boolean {
  return (
    api !== null &&
    ['jsx', 'jsxs', 'jsxDEV', 'createElement'].includes(api.name)
  )
}

export function jsxName(
  node: TSESTree.JSXTagNameExpression,
  scopeOf: ScopeOf,
): Api | null {
  if (node.type === AST_NODE_TYPES.JSXIdentifier) return imported(node, scopeOf)
  if (
    node.type === AST_NODE_TYPES.JSXMemberExpression &&
    node.object.type === AST_NODE_TYPES.JSXIdentifier
  ) {
    const root = imported(node.object, scopeOf)
    return root?.name === '*'
      ? { source: root.source, name: node.property.name }
      : null
  }
  return null
}

export function styleComponent(
  input: TSESTree.Node,
  scopeOf: ScopeOf,
  seen = new Set<TSESTree.Node>(),
): boolean {
  const node = unwrap(input)
  if (seen.has(node)) return false
  if (component(imported(node, scopeOf))) return true
  if (
    node.type === AST_NODE_TYPES.CallExpression ||
    node.type === AST_NODE_TYPES.TaggedTemplateExpression
  )
    return styledFactory(
      node.type === AST_NODE_TYPES.CallExpression ? node.callee : node.tag,
      scopeOf,
    )
  if (
    node.type !== AST_NODE_TYPES.Identifier &&
    node.type !== AST_NODE_TYPES.JSXIdentifier
  )
    return false
  const init = constant(node, scopeOf)
  return init !== null && styleComponent(init, scopeOf, new Set(seen).add(node))
}

export function argumentsOf(
  args: readonly TSESTree.Node[],
  scopeOf: ScopeOf,
): TSESTree.Node[] {
  const expand = (
    input: TSESTree.Node,
    seen = new Set<TSESTree.Node>(),
  ): TSESTree.Node[] => {
    const node = unwrap(input)
    if (seen.has(node)) return []
    const next = new Set(seen).add(node)
    if (node.type === AST_NODE_TYPES.Identifier) {
      const init = constant(node, scopeOf)
      return init ? expand(init, next) : []
    }
    if (node.type !== AST_NODE_TYPES.ArrayExpression) return []
    return node.elements.flatMap((element) =>
      element === null
        ? []
        : element.type === AST_NODE_TYPES.SpreadElement
          ? expand(element.argument, next)
          : [element],
    )
  }
  return args.flatMap((argument) =>
    argument.type === AST_NODE_TYPES.SpreadElement
      ? expand(argument.argument)
      : [argument],
  )
}
