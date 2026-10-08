import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import type { ImportStorage } from './import-storage'
import { componentName, type StyleSite, styleValueSite } from './style-position'

/** Responsive values belong to a property, not to a composition argument. */
export function responsiveValueSite(
  node: TSESTree.Node,
  imports: ImportStorage,
): StyleSite | null {
  const site = styleValueSite(node, imports)
  if (!site) return null
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
