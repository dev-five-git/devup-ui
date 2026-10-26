import {
  bindingScope,
  field,
  type GuardNode,
  type GuardScope,
  nodeName,
} from './guard-nodes'

export function localCallable(
  node: GuardNode | undefined,
  locals: ReadonlyMap<string, GuardNode>,
): boolean {
  let current = node
  let seen: Set<string> | undefined
  while (current) {
    switch (current.kind) {
      case 'FunctionDeclaration': {
        const name = nodeName(field(current, 'id'))
        return !name || locals.get(name) === current
      }
      case 'FunctionExpression':
      case 'ArrowFunctionExpression':
        return true
      case 'Identifier': {
        const name = nodeName(current)
        seen ??= new Set()
        if (seen.has(name)) return false
        seen.add(name)
        current = locals.get(name)
        break
      }
      default:
        return false
    }
  }
  return false
}

export function callableLocals(
  statement: GuardNode,
  locals: Map<string, GuardNode>,
): void {
  const declaration = [
    'ExportNamedDeclaration',
    'ExportDefaultDeclaration',
  ].includes(statement.kind)
    ? field(statement, 'declaration')
    : statement
  if (declaration?.kind === 'FunctionDeclaration')
    locals.set(nodeName(field(declaration, 'id')), declaration)
  if (
    declaration?.kind === 'VariableDeclaration' &&
    declaration.value === 'const'
  ) {
    for (const item of declaration.fields.get('declarations') ?? []) {
      const id = field(item, 'id')
      const init = field(item, 'init')
      if (id?.kind === 'Identifier' && init) locals.set(id.name, init)
    }
  }
}

export function callableWrites(
  node: GuardNode,
  scope: GuardScope,
  locals: Map<string, GuardNode>,
): void {
  let pattern: GuardNode | undefined
  switch (node.kind) {
    case 'CallExpression': {
      const callee = field(node, 'callee')
      if (
        callee?.kind === 'Identifier' &&
        callee.name === 'eval' &&
        !node.optional &&
        !bindingScope(scope, 'eval')
      ) {
        for (const [name, declaration] of locals) {
          if (declaration.kind !== 'FunctionDeclaration') continue
          const owner = bindingScope(scope, name)
          if (owner && !owner.parent) locals.delete(name)
        }
      }
      return
    }
    case 'AssignmentExpression':
    case 'ForOfStatement':
    case 'ForInStatement':
      pattern = field(node, 'left')
      break
    case 'UpdateExpression':
      pattern = field(node, 'argument')
      break
    default:
      return
  }
  function invalidate(target: GuardNode | undefined): void {
    if (!target) return
    switch (target.kind) {
      case 'Identifier': {
        const owner = bindingScope(scope, target.name)
        if (owner && !owner.parent) locals.delete(target.name)
        return
      }
      case 'Property':
        invalidate(field(target, 'value'))
        return
      case 'AssignmentPattern':
        invalidate(field(target, 'left'))
        return
      case 'RestElement':
        invalidate(field(target, 'argument'))
        return
      case 'ObjectPattern':
      case 'ArrayPattern':
        for (const child of target.fields.get(
          target.kind === 'ObjectPattern' ? 'properties' : 'elements',
        ) ?? [])
          invalidate(child)
    }
  }
  invalidate(pattern)
}
