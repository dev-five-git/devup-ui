import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import type { ImportStorage } from './import-storage'
import { componentName, type StyleSite, styleValueSite } from './style-position'

/** The binding `node` calls or reads through: `css` of `css(...)`, `styled` of `styled.div.attrs(...)(...)` */
function calleeBinding(node: TSESTree.Node): TSESTree.Node {
  if (node.type === AST_NODE_TYPES.MemberExpression)
    return calleeBinding(node.object)
  if (node.type === AST_NODE_TYPES.CallExpression)
    return calleeBinding(node.callee)
  return node
}

/** The package of another library the build reads as Devup UI whose API `root` is. Its rules declare a property's array of literals as that library does (`color: ['red', 'blue']`), as `library_array` in `libs/extractor/src/library_arrays.rs` writes it, not as breakpoints */
export function fallbackRulesLibrary(
  root: TSESTree.JSXOpeningElement | TSESTree.CallExpression,
  imports: ImportStorage,
): string | undefined {
  const binding =
    root.type === AST_NODE_TYPES.CallExpression
      ? calleeBinding(root.callee)
      : root.name
  return binding.type === AST_NODE_TYPES.Identifier ||
    binding.type === AST_NODE_TYPES.JSXIdentifier
    ? imports.fallbackLibrary(binding.name)
    : undefined
}

/** Responsive values belong to a property, not to a composition argument. */
export function responsiveValueSite(
  node: TSESTree.Node,
  imports: ImportStorage,
): StyleSite | null {
  const site = styleValueSite(node, imports)
  if (!site || fallbackRulesLibrary(site.root, imports)) return null
  let child = node
  let nestedArray = false
  while (child !== site.start && child.parent) {
    const parent = child.parent
    if (parent.type === AST_NODE_TYPES.ArrayExpression) nestedArray = true
    if (parent.type === AST_NODE_TYPES.Property)
      return nestedArray ? null : site
    if (parent.type === AST_NODE_TYPES.JSXAttribute)
      return nestedArray ||
        componentName(parent.parent.name, imports) === 'Global'
        ? null
        : site
    child = parent
  }
  if (site.start === site.root) return null
  const declarator = site.start.parent
  if (declarator?.type !== AST_NODE_TYPES.VariableDeclarator) return null
  const [variable] = imports.declaredVariables(declarator)
  return variable.references
    .filter((reference) => !reference.init)
    .every((reference) => responsiveValueSite(reference.identifier, imports))
    ? site
    : null
}
