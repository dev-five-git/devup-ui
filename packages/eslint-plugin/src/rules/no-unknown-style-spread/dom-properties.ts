import type {
  Declaration,
  ExportAssignment,
  Type,
  TypeChecker,
} from 'typescript'

export function domProperties(checker: TypeChecker, element: Type) {
  const declarations = element.getSymbol()?.getDeclarations() ?? []
  const parent = declarations
    .map((node) => node.parent?.parent)
    .find((candidate) =>
      candidate
        ?.getSourceFile()
        .statements.filter(
          (statement): statement is ExportAssignment =>
            'isExportEquals' in statement && statement.isExportEquals === true,
        )
        .some((statement) =>
          checker
            .getSymbolAtLocation(statement.expression)
            ?.getDeclarations()
            ?.some((declaration) => declaration === candidate),
        ),
    )
  if (!parent) return undefined
  const originalFile = parent.getSourceFile()
  const namespace = parent
    .getChildren()
    .map((node) => checker.getSymbolAtLocation(node))
    .find((symbol) =>
      symbol?.getDeclarations()?.some((declaration) => declaration === parent),
    )
  const jsx =
    namespace &&
    checker
      .getExportsOfModule(namespace)
      .find((symbol) => symbol.name === 'JSX')
  const intrinsic =
    jsx &&
    checker
      .getExportsOfModule(jsx)
      .find((symbol) => symbol.name === 'IntrinsicElements')
  if (!intrinsic) return undefined
  const result = new Set<Declaration>()
  for (const tag of checker.getPropertiesOfType(
    checker.getDeclaredTypeOfSymbol(intrinsic),
  )) {
    const origins = tag.getDeclarations() ?? []
    if (
      !origins.length ||
      !origins.some((node) => node.getSourceFile() === originalFile)
    )
      continue
    const props = checker.getTypeOfSymbolAtLocation(tag, parent)
    for (const property of checker.getPropertiesOfType(props)) {
      for (const declaration of property.getDeclarations() ?? []) {
        if (declaration.getSourceFile() === originalFile)
          result.add(declaration)
      }
    }
  }
  return result
}
