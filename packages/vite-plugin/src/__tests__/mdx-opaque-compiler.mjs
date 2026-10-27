function replaceDefault() {
  return (tree) => {
    tree.body = tree.body.filter(
      (node) => node.type !== 'ExportDefaultDeclaration',
    )
    tree.body.push({
      type: 'ExportNamedDeclaration',
      source: { type: 'Literal', value: '@devup-ui/react' },
      specifiers: [
        {
          type: 'ExportSpecifier',
          local: { type: 'Identifier', name: 'Box' },
          exported: { type: 'Identifier', name: 'default' },
        },
      ],
    })
  }
}
function preserveDefault() {
  return (tree) => {
    for (const statement of tree.body)
      if (
        statement.type === 'ExportDefaultDeclaration' &&
        statement.declaration.type === 'FunctionDeclaration'
      )
        statement.declaration.id.name = 'PreservedContent'
  }
}
export function compilerPlugins(kind) {
  switch (kind) {
    case 'replacement':
    case 'replacement-string':
    case 'replacement-namespace':
      return [replaceDefault]
    case 'preserved':
      return [preserveDefault]
    default:
      return []
  }
}

export function defaultSyntaxPlugin(kind, description) {
  return {
    name: 'configured-default-export-syntax',
    enforce: 'post',
    transform(code, id) {
      if (
        id !== description ||
        !['replacement-string', 'replacement-namespace'].includes(kind)
      )
        return
      const changed = code.replace(
        /\{\s*Box\s+as\s+default\s*\}/,
        kind === 'replacement-string' ? '{Box as "default"}' : '* as default',
      )
      if (changed === code)
        throw new TypeError('Expected actual recma-generated default reexport')
      const statements = this.parse(changed).body.filter((statement) =>
        statement.type.startsWith('Export'),
      )
      console.info(
        JSON.stringify({
          event: 'configured-default-export-syntax',
          kind,
          exports: statements.map((statement) => ({
            type: statement.type,
            specifiers: statement.specifiers,
            exported: statement.exported,
          })),
        }),
      )
      return changed
    },
  }
}
