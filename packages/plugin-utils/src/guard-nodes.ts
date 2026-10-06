export interface GuardNode {
  readonly kind: string
  readonly name: string
  readonly value: string
  readonly pos: number
  readonly fields: ReadonlyMap<string, readonly GuardNode[]>
  readonly computed: boolean
}

export function guardNode(value: unknown): GuardNode | undefined {
  if (
    !value ||
    typeof value !== 'object' ||
    !('type' in value) ||
    typeof value.type !== 'string'
  )
    return
  const fields = new Map<string, readonly GuardNode[]>()
  for (const [key, child] of Object.entries(value)) {
    const children = (Array.isArray(child) ? child : [child]).flatMap(
      (item: unknown) => {
        const node = guardNode(item)
        return node ? [node] : []
      },
    )
    if (children.length) fields.set(key, children)
  }
  return {
    kind: value.type,
    name: 'name' in value && typeof value.name === 'string' ? value.name : '',
    value:
      'value' in value && typeof value.value === 'string'
        ? value.value
        : 'kind' in value && typeof value.kind === 'string'
          ? value.kind
          : '',
    pos: 'start' in value && typeof value.start === 'number' ? value.start : 0,
    computed: 'computed' in value && value.computed === true,
    fields,
  }
}

export function field(node: GuardNode, key: string): GuardNode | undefined {
  return node.fields.get(key)?.[0]
}

export function nodeName(node: GuardNode | undefined): string {
  return node?.name || node?.value || ''
}

export interface GuardOrigin {
  readonly request: string
  readonly ids: readonly string[]
  readonly kind?: 'require-call'
}

export interface GuardScope {
  readonly parent?: GuardScope
  readonly functionScope: boolean
  readonly bindings: Map<string, GuardOrigin | undefined>
}

export function binding(
  scope: GuardScope,
  name: string,
): GuardOrigin | undefined {
  return bindingScope(scope, name)?.bindings.get(name)
}

export function bindingScope(
  scope: GuardScope,
  name: string,
): GuardScope | undefined {
  return scope.bindings.has(name)
    ? scope
    : scope.parent && bindingScope(scope.parent, name)
}

export function patternNames(node: GuardNode): string[] {
  if (node.kind === 'Identifier') return [node.name]
  if (node.kind === 'Property') {
    const value = field(node, 'value')
    return value ? patternNames(value) : []
  }
  if (node.kind === 'AssignmentPattern') {
    const left = field(node, 'left')
    return left ? patternNames(left) : []
  }
  return [...node.fields.values()].flat().flatMap(patternNames)
}

export function createScopes(root: GuardNode) {
  const scopes = new Map<GuardNode, GuardScope>()
  const moduleScope: GuardScope = { functionScope: true, bindings: new Map() }
  function visit(node: GuardNode, parent: GuardScope) {
    const functionScope =
      /^(?:Program|FunctionDeclaration|FunctionExpression|ArrowFunctionExpression)$/.test(
        node.kind,
      )
    const classScope = ['ClassDeclaration', 'ClassExpression'].includes(
      node.kind,
    )
    const scope =
      node === root
        ? moduleScope
        : functionScope ||
            classScope ||
            [
              'BlockStatement',
              'CatchClause',
              'ForStatement',
              'ForOfStatement',
              'ForInStatement',
              'StaticBlock',
              'SwitchStatement',
            ].includes(node.kind)
          ? {
              parent,
              functionScope,
              bindings: new Map<string, GuardOrigin | undefined>(),
            }
          : parent
    scopes.set(node, scope)
    const id = field(node, 'id')
    if (id && ['FunctionDeclaration', 'ClassDeclaration'].includes(node.kind))
      parent.bindings.set(id.name, undefined)
    if (functionScope || classScope) {
      if (id) scope.bindings.set(id.name, undefined)
      for (const parameter of node.fields.get('params') ?? [])
        for (const name of patternNames(parameter))
          scope.bindings.set(name, undefined)
    }
    const param = field(node, 'param')
    if (param)
      for (const name of patternNames(param))
        scope.bindings.set(name, undefined)
    if (node.kind === 'VariableDeclaration') {
      let target = scope
      if (node.value === 'var')
        while (!target.functionScope && target.parent) target = target.parent
      for (const declaration of node.fields.get('declarations') ?? []) {
        const pattern = field(declaration, 'id')
        if (pattern)
          for (const name of patternNames(pattern))
            target.bindings.set(name, undefined)
      }
    }
    for (const children of node.fields.values())
      for (const child of children) visit(child, scope)
  }
  visit(root, moduleScope)
  return { scopes, moduleScope }
}
