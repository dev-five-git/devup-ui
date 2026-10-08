import {
  AST_NODE_TYPES,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

const DEVUP_PACKAGE = '@devup-ui/react'

/** The packages whose default export, and the namespace they are imported as, the build reads as the named export `styled` of Devup UI, as `DEFAULT_IMPORT_ALIASES` in `plugin-utils` lists them */
const DEFAULT_IS_STYLED = new Set(['@emotion/styled', 'styled-components'])

/** The packages whose named exports the build maps onto Devup UI's, as `DEFAULT_IMPORT_ALIASES` in `plugin-utils` lists them */
const NAMED_ALIASES = new Set([
  '@emotion/react',
  '@emotion/styled',
  'styled-components',
  '@vanilla-extract/css',
])

/** The names of another package the build reads as Devup UI's, as `devup_equivalent` in `libs/extractor/src/import_alias_visit.rs` maps them */
function devupEquivalent(source: string, imported: string): string | undefined {
  if (source === '@vanilla-extract/css') {
    if (imported === 'style') return 'css'
    if (imported === 'globalStyle') return 'globalCss'
  }
  switch (imported) {
    case 'css':
    case 'keyframes':
    case 'styled':
    case 'createGlobalStyle':
    case 'Global':
    case 'ThemeProvider':
    case 'ServerStyleSheet':
    case 'StyleSheetManager':
    case 'isStyledComponent':
    case 'withTheme':
    case 'useTheme':
      return imported
    default:
      return undefined
  }
}

/** The components the build compiles into elements */
export const STYLE_COMPONENTS: ReadonlySet<string> = new Set([
  'Box',
  'Button',
  'Center',
  'Flex',
  'Grid',
  'Image',
  'Input',
  'Text',
  'VStack',
])

/** The functions the build compiles into the classes and names they give */
export const STYLE_FUNCTIONS: ReadonlySet<string> = new Set([
  'css',
  'globalCss',
  'keyframes',
  'createGlobalStyle',
])

/** Whether `filename` is a vanilla-extract stylesheet, which the build runs as a whole instead of reading its calls, as `is_vanilla_extract_file` does */
export function isVanillaExtractFile(filename: string): boolean {
  return filename.endsWith('.css.ts') || filename.endsWith('.css.js')
}

export interface StorageContext {
  readonly filename: string
  readonly sourceCode: Readonly<TSESLint.SourceCode>
}

/** What a file imports from Devup UI and from the packages the build reads as it */
export class ImportStorage {
  private imports = new Map<string, string>()
  private importObject = new Set<string>()
  readonly vanilla: boolean

  constructor(private readonly context?: StorageContext) {
    this.vanilla = isVanillaExtractFile(context?.filename ?? '')
  }

  /** The variables `node` declares */
  public declaredVariables(
    node: TSESTree.Node,
  ): readonly TSESLint.Scope.Variable[] {
    return this.context?.sourceCode.getDeclaredVariables(node) ?? []
  }

  public addImportByDeclaration(node: TSESTree.ImportDeclaration) {
    const source = node.source.value
    // A vanilla-extract stylesheet runs as it is: no import is compiled away
    if (this.vanilla) return
    const isDevup =
      source === DEVUP_PACKAGE || source === `${DEVUP_PACKAGE}/compat`
    if (
      !isDevup &&
      !NAMED_ALIASES.has(source) &&
      !DEFAULT_IS_STYLED.has(source)
    )
      return

    for (const specifier of node.specifiers) {
      switch (specifier.type) {
        case AST_NODE_TYPES.ImportSpecifier: {
          const imported =
            specifier.imported.type === AST_NODE_TYPES.Literal
              ? specifier.imported.value
              : specifier.imported.name
          const name = isDevup ? imported : devupEquivalent(source, imported)
          if (name !== undefined) this.addImport(specifier.local.name, name)
          break
        }
        case AST_NODE_TYPES.ImportDefaultSpecifier:
        case AST_NODE_TYPES.ImportNamespaceSpecifier:
          if (isDevup) this.importObject.add(specifier.local.name)
          else if (DEFAULT_IS_STYLED.has(source))
            this.addImport(specifier.local.name, 'styled')
          break
      }
    }
  }

  public addImport(key: string, value: string) {
    this.imports.set(key, value)
  }

  /** The name Devup UI exports for what `local` imports by name, from it or from a package the build reads as it */
  public importedName(local: string): string | undefined {
    return this.imports.get(local)
  }

  /** Whether `local` is the package imported whole, as a namespace or default */
  public isImportObject(local: string): boolean {
    return this.importObject.has(local)
  }

  /** The names the file binds to what the build compiles away, with the Devup UI name each stands for */
  public bindings(): [string, string][] {
    return [...this.imports]
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
    const callee = node.callee
    if (callee.type === AST_NODE_TYPES.Identifier)
      return this.importedName(callee.name) !== undefined
    return (
      callee.type === AST_NODE_TYPES.MemberExpression &&
      !callee.computed &&
      callee.object.type === AST_NODE_TYPES.Identifier &&
      this.importObject.has(callee.object.name) &&
      callee.property.type === AST_NODE_TYPES.Identifier &&
      (STYLE_FUNCTIONS.has(callee.property.name) ||
        callee.property.name === 'styled')
    )
  }

  private checkDevupUIComponent(node: TSESTree.JSXTagNameExpression): boolean {
    return (
      (node.type === AST_NODE_TYPES.JSXIdentifier &&
        this.importedName(node.name) !== undefined) ||
      (node.type === AST_NODE_TYPES.JSXMemberExpression &&
        node.object.type === AST_NODE_TYPES.JSXIdentifier &&
        this.importObject.has(node.object.name) &&
        STYLE_COMPONENTS.has(node.property.name))
    )
  }
}
