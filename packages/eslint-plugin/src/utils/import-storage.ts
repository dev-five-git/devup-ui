import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'
const defaultImports = {
  css: 'css',
  globalCss: 'globalCss',
  keyframes: 'keyframes',
  Box: 'Box',
  Button: 'Button',
  Text: 'Text',
  Image: 'Image',
  Flex: 'Flex',
  Grid: 'Grid',
  Center: 'Center',
  VStack: 'VStack',
  Input: 'Input',
}

export class ImportStorage {
  private imports: Record<string, string>
  private importObject: Set<string>

  constructor() {
    this.imports = {}
    this.importObject = new Set()
  }

  public addImportByDeclaration(node: TSESTree.ImportDeclaration) {
    if (node.source.value !== '@devup-ui/react') return

    for (const specifier of node.specifiers) {
      switch (specifier.type) {
        case AST_NODE_TYPES.ImportSpecifier:
          this.addImport(
            specifier.local.name,
            specifier.imported.type === AST_NODE_TYPES.Literal
              ? specifier.imported.value
              : specifier.imported.name,
          )
          break
        case AST_NODE_TYPES.ImportDefaultSpecifier:
          this.importObject.add(specifier.local.name)
          break
        case AST_NODE_TYPES.ImportNamespaceSpecifier:
          this.importObject.add(specifier.local.name)
          break
      }
    }
  }

  public addImport(key: string, value: string) {
    this.imports[key] = value
  }

  public checkContextType(node: TSESTree.Node) {
    switch (node.type) {
      case AST_NODE_TYPES.JSXOpeningElement: {
        if (this.checkDevupUIComponent(node.name)) {
          return 'COMPONENT'
        }
        break
      }
      case AST_NODE_TYPES.CallExpression: {
        if (this.checkDevupUIUtil(node)) {
          return 'UTIL'
        }
        break
      }
    }
  }

  private checkDevupUIUtil(node: TSESTree.CallExpression): boolean {
    return (
      (node.callee.type === AST_NODE_TYPES.Identifier &&
        node.callee.name in this.imports) ||
      (node.callee.type === AST_NODE_TYPES.MemberExpression &&
        node.callee.object.type === AST_NODE_TYPES.Identifier &&
        this.importObject.has(node.callee.object.name) &&
        node.callee.property.type === AST_NODE_TYPES.Identifier &&
        node.callee.property.name in defaultImports)
    )
  }

  private checkDevupUIComponent(node: TSESTree.JSXTagNameExpression): boolean {
    return (
      (node.type === AST_NODE_TYPES.JSXIdentifier &&
        node.name in this.imports) ||
      (node.type === AST_NODE_TYPES.JSXMemberExpression &&
        node.object.type === AST_NODE_TYPES.JSXIdentifier &&
        this.importObject.has(node.object.name) &&
        node.property.name in defaultImports)
    )
  }
}
