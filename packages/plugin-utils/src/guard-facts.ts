import {
  bindingScope,
  createScopes,
  field,
  type GuardNode,
  guardNode,
  type GuardOrigin,
  nodeName,
  patternNames,
} from './guard-nodes'
import { origin, patternExpressions } from './guard-origins'

export interface GuardFacts {
  readonly references: readonly (GuardOrigin & { readonly pos: number })[]
  readonly exports: ReadonlyMap<string, GuardOrigin | undefined>
  readonly stars: readonly string[]
}

export function compiledFacts(ast: unknown): GuardFacts {
  const root = guardNode(ast)
  const references: (GuardOrigin & { readonly pos: number })[] = []
  const exports = new Map<string, GuardOrigin | undefined>()
  const stars: string[] = []
  if (!root) return { references, exports, stars }
  const { scopes, moduleScope } = createScopes(root)
  for (const statement of root.fields.get('body') ?? []) {
    if (statement.kind !== 'ImportDeclaration') continue
    const request = nodeName(field(statement, 'source'))
    for (const specifier of statement.fields.get('specifiers') ?? []) {
      const local = field(specifier, 'local')
      if (!local) continue
      const ids =
        specifier.kind === 'ImportNamespaceSpecifier'
          ? []
          : [
              specifier.kind === 'ImportDefaultSpecifier'
                ? 'default'
                : nodeName(field(specifier, 'imported')),
            ]
      moduleScope.bindings.set(local.name, { request, ids })
    }
  }
  function record(value: GuardOrigin | undefined, pos: number) {
    if (value?.ids.length) references.push({ ...value, pos })
  }
  function visit(node: GuardNode, key = '') {
    const scope = scopes.get(node) ?? moduleScope
    if (node.kind === 'VariableDeclaration' && node.value === 'const') {
      for (const declaration of node.fields.get('declarations') ?? []) {
        const id = field(declaration, 'id')
        const value = origin(field(declaration, 'init'), scope)
        if (id?.kind === 'Identifier' && value)
          bindingScope(scope, id.name)?.bindings.set(id.name, value)
      }
    }
    if (node.kind === 'ImportDeclaration') return
    if (node.kind === 'ExportAllDeclaration') {
      const request = nodeName(field(node, 'source'))
      const exported = nodeName(field(node, 'exported'))
      if (exported) exports.set(exported, { request, ids: [] })
      else stars.push(request)
      return
    }
    if (node.kind === 'ExportNamedDeclaration') {
      const request = nodeName(field(node, 'source'))
      for (const specifier of node.fields.get('specifiers') ?? []) {
        const name = nodeName(field(specifier, 'exported'))
        const local = field(specifier, 'local')
        exports.set(
          name,
          request ? { request, ids: [nodeName(local)] } : origin(local, scope),
        )
      }
      const declaration = field(node, 'declaration')
      if (declaration) {
        const ids =
          declaration.kind === 'VariableDeclaration'
            ? (declaration.fields.get('declarations') ?? []).flatMap((item) => {
                const pattern = field(item, 'id')
                return pattern ? patternNames(pattern) : []
              })
            : [nodeName(field(declaration, 'id'))]
        for (const name of ids) exports.set(name, undefined)
      }
    }
    if (node.kind === 'ExportDefaultDeclaration')
      exports.set('default', origin(field(node, 'declaration'), scope))
    if (['VariableDeclarator', 'AssignmentExpression'].includes(node.kind)) {
      const pattern = field(
        node,
        node.kind === 'VariableDeclarator' ? 'id' : 'left',
      )
      const value = origin(
        field(node, node.kind === 'VariableDeclarator' ? 'init' : 'right'),
        scope,
      )
      if (pattern?.kind === 'ObjectPattern' && value) {
        for (const property of pattern.fields.get('properties') ?? []) {
          const propertyKey = field(property, 'key')
          if (
            property.kind !== 'Property' ||
            (property.computed && propertyKey?.kind !== 'Literal')
          )
            continue
          record(
            {
              ...value,
              ids: [...value.ids, nodeName(propertyKey)],
            },
            property.pos,
          )
        }
      }
    }
    if (node.kind === 'MemberExpression') {
      record(origin(node, scope), node.pos)
      const object = field(node, 'object')
      if (object && !origin(object, scope)) visit(object, 'object')
      const property = field(node, 'property')
      if (node.computed && property?.kind !== 'Literal' && property)
        visit(property, 'property')
      return
    }
    if (node.kind === 'Identifier') {
      if (
        [
          'id',
          'params',
          'param',
          'imported',
          'exported',
          'local',
          'key',
          'label',
        ].includes(key)
      )
        return
      record(origin(node, scope), node.pos)
      return
    }
    if (
      [
        'ObjectPattern',
        'ArrayPattern',
        'AssignmentPattern',
        'RestElement',
      ].includes(node.kind)
    ) {
      for (const expression of patternExpressions(node)) visit(expression)
      return
    }
    for (const [name, children] of node.fields)
      for (const child of children) visit(child, name)
  }
  visit(root)
  return { references, exports, stars }
}
