import {
  AST_NODE_TYPES,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

const DEVUP_PACKAGE = '@devup-ui/react'

/** The key of the shared ESLint settings that mirror the build plugin's options */
export const SETTINGS_KEY = '@devup-ui'

/**
 * The packages the build reads as Devup UI by default, as `DEFAULT_IMPORT_ALIASES` in `plugin-utils` lists them.
 * A string names the Devup UI export that the default export and the namespace stand for; `null` maps the named exports only.
 */
const DEFAULT_IMPORT_ALIASES: ReadonlyMap<string, string | null> = new Map([
  ['@emotion/react', null],
  ['@emotion/styled', 'styled'],
  ['@emotion/css', null],
  ['styled-components', 'styled'],
  ['@vanilla-extract/css', null],
  ['@vanilla-extract/css-utils', null],
])

/**
 * The exports of each library whose rules declare a property's array of literals as fallbacks in one rule (`color: ['red', 'blue']`), not breakpoints, as `LibraryNumbers` in `libs/extractor/src/import_alias_visit.rs` lists them and `library_array` in `libs/extractor/src/library_arrays.rs` writes them.
 * `default` stands for the default and namespace imports. Other aliases read rules as Devup UI does.
 */
const FALLBACK_EXPORTS: ReadonlyMap<string, ReadonlySet<string>> = new Map([
  ['@emotion/react', new Set(['css', 'keyframes', 'Global'])],
  ['@emotion/styled', new Set(['styled', 'default'])],
  ['@emotion/css', new Set(['css', 'keyframes', 'injectGlobal'])],
  ['styled-components', new Set(['css', 'keyframes', 'styled', 'default'])],
  ['@vanilla-extract/css', new Set(['style', 'keyframes', 'globalStyle'])],
])

function takesFallbacks(source: string, imported: string): boolean {
  return FALLBACK_EXPORTS.get(source)?.has(imported) ?? false
}

/** The package whose `calc` the build writes as the string it gives, as `vanilla_extract_calc` in `libs/extractor` compiles it */
const CSS_UTILS = '@vanilla-extract/css-utils'

/** The name `specifier` imports */
function specifierName(specifier: TSESTree.ImportSpecifier): string {
  return specifier.imported.type === AST_NODE_TYPES.Literal
    ? specifier.imported.value
    : specifier.imported.name
}

/** What the build is configured to read as Devup UI */
export interface ImportConfig {
  readonly package: string
  readonly aliases: ReadonlyMap<string, string | null>
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function settingsError(path: string, expected: string): TypeError {
  return new TypeError(
    `@devup-ui/eslint-plugin: settings["${SETTINGS_KEY}"]${path} must be ${expected}, as the build plugin option of the same name`,
  )
}

/** The package the build compiles away, from the `package` setting */
function readPackage(value: unknown): string {
  if (value === undefined) return DEVUP_PACKAGE
  if (typeof value !== 'string' || value === '')
    throw settingsError('.package', 'a package name')
  return value
}

/** The default aliases merged with the `importAliases` setting, as `mergeImportAliases` in `plugin-utils` merges them */
function readAliases(value: unknown): ReadonlyMap<string, string | null> {
  if (value === undefined) return DEFAULT_IMPORT_ALIASES
  if (!isRecord(value))
    throw settingsError('.importAliases', 'an object of package names')
  const aliases = new Map(DEFAULT_IMPORT_ALIASES)
  for (const [source, alias] of Object.entries(value)) {
    if (alias === false) aliases.delete(source)
    else if (alias === true) aliases.set(source, null)
    else if (typeof alias === 'string' && alias !== '')
      aliases.set(source, alias)
    else
      throw settingsError(
        `.importAliases[${JSON.stringify(source)}]`,
        'an export name, true or false',
      )
  }
  return aliases
}

/** What the shared `settings` configure the build to read as Devup UI */
export function importConfig(settings: unknown): ImportConfig {
  const devup = isRecord(settings) ? settings[SETTINGS_KEY] : undefined
  if (devup === undefined)
    return { package: DEVUP_PACKAGE, aliases: DEFAULT_IMPORT_ALIASES }
  if (!isRecord(devup)) throw settingsError('', 'an object')
  return {
    package: readPackage(devup.package),
    aliases: readAliases(devup.importAliases),
  }
}

/** The names of another package the build reads as Devup UI's, as `devup_equivalent` in `libs/extractor/src/import_alias_visit.rs` maps them */
function devupEquivalent(source: string, imported: string): string | undefined {
  if (source === '@vanilla-extract/css') {
    if (imported === 'style') return 'css'
    if (imported === 'globalStyle') return 'globalCss'
  }
  if (source === '@emotion/css' && imported === 'injectGlobal')
    return 'globalCss'
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
  /** The shared settings, whose `@devup-ui` entry mirrors the build plugin's `package` and `importAliases` options */
  readonly settings?: unknown
}

/** What a file imports from Devup UI and from the packages the build reads as it */
export class ImportStorage {
  private imports = new Map<string, string>()
  private importObject = new Set<string>()
  private calc = new Set<string>()
  /** The package of each binding whose rules take arrays as fallbacks */
  private fallbacks = new Map<string, string>()
  readonly vanilla: boolean
  private readonly config: ImportConfig

  constructor(private readonly context?: StorageContext) {
    this.vanilla = isVanillaExtractFile(context?.filename ?? '')
    this.config = importConfig(context?.settings)
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
    if (source === CSS_UTILS) {
      // Turned off with `importAliases`, the build leaves `calc` to the package
      if (!this.config.aliases.has(CSS_UTILS)) return
      for (const specifier of node.specifiers)
        if (
          specifier.type === AST_NODE_TYPES.ImportSpecifier &&
          specifierName(specifier) === 'calc'
        )
          this.calc.add(specifier.local.name)
      return
    }
    const isDevup =
      source === this.config.package ||
      source === `${this.config.package}/compat`
    const alias = this.config.aliases.get(source)
    if (!isDevup && alias === undefined) return

    for (const specifier of node.specifiers) {
      switch (specifier.type) {
        case AST_NODE_TYPES.ImportSpecifier: {
          const imported = specifierName(specifier)
          const name = isDevup ? imported : devupEquivalent(source, imported)
          this.addBinding(
            specifier.local.name,
            name,
            !isDevup && takesFallbacks(source, imported) ? source : undefined,
          )
          break
        }
        case AST_NODE_TYPES.ImportDefaultSpecifier:
        case AST_NODE_TYPES.ImportNamespaceSpecifier: {
          if (isDevup) {
            this.importObject.add(specifier.local.name)
            break
          }
          // Both stand for the one export a string alias names, as `DefaultToNamed` in the build
          const name = alias ? devupEquivalent(source, alias) : undefined
          this.addBinding(
            specifier.local.name,
            name,
            takesFallbacks(source, 'default') ? source : undefined,
          )
          break
        }
      }
    }
  }

  /** Binds `local` to the Devup UI export `name` it stands for, if it stands for one, remembering the package whose rules it takes when they take arrays as fallbacks */
  private addBinding(
    local: string,
    name: string | undefined,
    fallbacks: string | undefined,
  ) {
    if (name === undefined) return
    this.addImport(local, name)
    if (fallbacks) this.fallbacks.set(local, fallbacks)
  }

  public addImport(key: string, value: string) {
    this.imports.set(key, value)
  }

  /** The package (`@emotion/react`, `styled-components`...) whose rules `local` takes, when they declare a property's array of literals as that library does rather than as breakpoints */
  public fallbackLibrary(local: string): string | undefined {
    return this.fallbacks.get(local)
  }

  /** The name Devup UI exports for what `local` imports by name, from it or from a package the build reads as it */
  public importedName(local: string): string | undefined {
    return this.imports.get(local)
  }

  /** Whether `local` binds the `calc` of `@vanilla-extract/css-utils`, whose calls the build writes as the string they give */
  public isCalc(local: string): boolean {
    return this.calc.has(local)
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
