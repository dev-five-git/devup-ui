import {
  binding,
  bindingScope,
  field,
  type GuardNode,
  type GuardOrigin,
  type GuardScope,
  nodeName,
} from './guard-nodes'

export function origin(
  node: GuardNode | undefined,
  scope: GuardScope,
): GuardOrigin | undefined {
  if (!node) return
  if (node.kind === 'Identifier') return binding(scope, node.name)
  if (node.kind === 'CallExpression') {
    const callee = field(node, 'callee')
    const args = node.fields.get('arguments') ?? []
    const request = args[0]
    if (
      callee?.kind === 'Identifier' &&
      callee.name === 'require' &&
      !bindingScope(scope, 'require') &&
      args.length === 1 &&
      request?.kind === 'Literal' &&
      request.value
    )
      return { request: request.value, ids: [], kind: 'require-call' }
  }
  if (node.kind === 'MemberExpression') {
    const object = origin(field(node, 'object'), scope)
    const property = field(node, 'property')
    if (object && property && (!node.computed || property.kind === 'Literal'))
      return { ...object, ids: [...object.ids, nodeName(property)] }
  }
}

/** Binding patterns are not reads; their defaults and computed keys are. */
export function patternExpressions(node: GuardNode): readonly GuardNode[] {
  if (node.kind === 'AssignmentPattern') {
    const left = field(node, 'left')
    const right = field(node, 'right')
    return [
      ...(left ? patternExpressions(left) : []),
      ...(right ? [right] : []),
    ]
  }
  if (node.kind === 'Property') {
    const key = field(node, 'key')
    const value = field(node, 'value')
    return [
      ...(node.computed && key ? [key] : []),
      ...(value ? patternExpressions(value) : []),
    ]
  }
  if (['ObjectPattern', 'ArrayPattern', 'RestElement'].includes(node.kind))
    return [...node.fields.values()].flat().flatMap(patternExpressions)
  return []
}
