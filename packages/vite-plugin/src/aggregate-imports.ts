export function inspectImports(
  ast: unknown,
  fail: (reason: string, position?: number) => never,
): boolean {
  if (!ast || typeof ast !== 'object') return false
  if (Array.isArray(ast))
    return ast.map((node) => inspectImports(node, fail)).some(Boolean)
  let imports = false
  if ('type' in ast && ast.type === 'ImportExpression') {
    const source = 'source' in ast ? ast.source : undefined
    if (!source || typeof source !== 'object' || !('type' in source)) {
      fail('public parser returned an incomplete dynamic import')
    }
    if (
      !(
        source.type === 'Literal' &&
        'value' in source &&
        typeof source.value === 'string'
      ) &&
      !(
        source.type === 'TemplateLiteral' &&
        'expressions' in source &&
        Array.isArray(source.expressions) &&
        source.expressions.length === 0
      )
    ) {
      fail(
        'computed dynamic import has no knowable resolved closure; replace it with literal imports or import.meta.glob',
        'start' in ast && typeof ast.start === 'number' ? ast.start : undefined,
      )
    }
    imports = true
  }
  if (
    'type' in ast &&
    (ast.type === 'ImportDeclaration' ||
      ast.type === 'ExportAllDeclaration' ||
      ast.type === 'ExportNamedDeclaration') &&
    'source' in ast &&
    ast.source
  )
    imports = true
  for (const value of Object.values(ast)) {
    if (inspectImports(value, fail)) imports = true
  }
  return imports
}
