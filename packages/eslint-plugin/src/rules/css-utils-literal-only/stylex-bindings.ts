import {
  AST_NODE_TYPES,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

const RULE_APIS = new Set([
  'create',
  'keyframes',
  'defineVars',
  'defineConsts',
  'createTheme',
  'createThemeContract',
  'positionTry',
  'viewTransitionClass',
])
const HELPERS = new Set([
  'props',
  'attrs',
  'firstThatWorks',
  'include',
  'types',
])
const TYPE_METHODS = new Set([
  'angle',
  'color',
  'image',
  'integer',
  'length',
  'lengthPercentage',
  'number',
  'percentage',
  'resolution',
  'time',
  'transformFunction',
  'transformList',
  'url',
])
type Source = 'root' | 'upstream' | 'dedicated' | 'other'
type Binding = 'root' | 'upstream' | 'namespace' | 'typeMethod' | string

function sourceOf(source: string): Source {
  switch (source) {
    case '@devup-ui/react':
      return 'root'
    case '@stylexjs/stylex':
      return 'upstream'
    case '@devup-ui/react/stylex':
      return 'dedicated'
    default:
      return 'other'
  }
}

function exported(source: Source, name: string): Binding | undefined {
  switch (source) {
    case 'root':
      return name === 'stylex' ? 'namespace' : undefined
    case 'upstream':
      if (name === 'default') return 'namespace'
      return RULE_APIS.has(name) || HELPERS.has(name) ? name : undefined
    case 'dedicated':
      return RULE_APIS.has(name) || HELPERS.has(name) ? name : undefined
    case 'other':
      return undefined
  }
}

function namespace(source: Source): Binding | undefined {
  switch (source) {
    case 'root':
      return 'root'
    case 'upstream':
      return 'upstream'
    case 'dedicated':
      return 'namespace'
    case 'other':
      return undefined
  }
}

function variableOf(scope: TSESLint.Scope.Scope, name: string) {
  for (
    let current: TSESLint.Scope.Scope | null = scope;
    current;
    current = current.upper
  ) {
    const variable = current.set.get(name)
    if (variable) return variable
  }
  return undefined
}

/** Resolves only advertised runtime bindings, using lexical identity rather than names. */
export class StylexBindings {
  constructor(
    private readonly scopeOf: (node: TSESTree.Node) => TSESLint.Scope.Scope,
  ) {}

  apiOf(node: TSESTree.Node): string | undefined {
    const binding = this.resolve(node, new Set())
    return binding && RULE_APIS.has(binding) ? binding : undefined
  }

  isCallee(node: TSESTree.Node): boolean {
    const binding = this.resolve(node, new Set())
    return (
      binding !== undefined &&
      (RULE_APIS.has(binding) ||
        HELPERS.has(binding) ||
        binding === 'typeMethod')
    )
  }

  private resolve(
    node: TSESTree.Node,
    seen: Set<TSESLint.Scope.Variable>,
  ): Binding | undefined {
    switch (node.type) {
      case AST_NODE_TYPES.TSAsExpression:
      case AST_NODE_TYPES.TSSatisfiesExpression:
      case AST_NODE_TYPES.TSNonNullExpression:
        return this.resolve(node.expression, seen)
      case AST_NODE_TYPES.MemberExpression: {
        if (
          node.computed ||
          node.optional ||
          node.property.type !== AST_NODE_TYPES.Identifier
        )
          return undefined
        const object = this.resolve(node.object, seen)
        const name = node.property.name
        switch (object) {
          case 'root':
            return name === 'stylex' ? 'namespace' : undefined
          case 'upstream':
            return name === 'default' ? 'namespace' : exported('upstream', name)
          case 'namespace':
            return exported('dedicated', name)
          case 'types':
            return TYPE_METHODS.has(name) ? 'typeMethod' : undefined
          default:
            return undefined
        }
      }
      case AST_NODE_TYPES.Identifier: {
        const variable = variableOf(this.scopeOf(node), node.name)
        if (!variable || seen.has(variable)) return undefined
        seen.add(variable)
        const definition = variable.defs[0]
        switch (definition?.type) {
          case 'ImportBinding':
            return this.imported(definition)
          case 'Variable': {
            const init = definition.node.init
            if (
              !init ||
              variable.references.some(
                (reference) => !reference.init && reference.isWrite(),
              )
            )
              return undefined
            const source = this.requiredSource(init)
            const id = definition.node.id
            if (id.type === AST_NODE_TYPES.Identifier) {
              const binding =
                source === undefined
                  ? this.resolve(init, seen)
                  : namespace(source)
              return binding === 'typeMethod' ? undefined : binding
            }
            if (
              source !== undefined &&
              id.type === AST_NODE_TYPES.ObjectPattern
            ) {
              for (const property of id.properties) {
                if (
                  property.type === AST_NODE_TYPES.Property &&
                  !property.computed &&
                  property.value.type === AST_NODE_TYPES.Identifier &&
                  property.value.name === node.name
                )
                  return exported(
                    source,
                    property.key.type === AST_NODE_TYPES.Identifier
                      ? property.key.name
                      : property.key.type === AST_NODE_TYPES.Literal &&
                          typeof property.key.value === 'string'
                        ? property.key.value
                        : '',
                  )
              }
            }
            return undefined
          }
          default:
            return undefined
        }
      }
      default:
        return undefined
    }
  }

  private imported(
    definition: Extract<TSESLint.Scope.Definition, { type: 'ImportBinding' }>,
  ): Binding | undefined {
    const declaration = definition.parent
    const source =
      declaration.type === AST_NODE_TYPES.ImportDeclaration &&
      declaration.importKind !== 'type'
        ? sourceOf(declaration.source.value)
        : 'other'
    const specifier = definition.node
    switch (specifier.type) {
      case AST_NODE_TYPES.ImportNamespaceSpecifier:
        return namespace(source)
      case AST_NODE_TYPES.ImportDefaultSpecifier:
        return exported(source, 'default')
      case AST_NODE_TYPES.ImportSpecifier:
        return specifier.importKind === 'type'
          ? undefined
          : exported(
              source,
              specifier.imported.type === AST_NODE_TYPES.Identifier
                ? specifier.imported.name
                : specifier.imported.value,
            )
      case AST_NODE_TYPES.TSImportEqualsDeclaration:
        return undefined
    }
  }

  private requiredSource(node: TSESTree.Node): Source | undefined {
    switch (node.type) {
      case AST_NODE_TYPES.TSAsExpression:
      case AST_NODE_TYPES.TSSatisfiesExpression:
      case AST_NODE_TYPES.TSNonNullExpression:
        return this.requiredSource(node.expression)
    }
    if (
      node.type !== AST_NODE_TYPES.CallExpression ||
      node.optional ||
      node.arguments.length !== 1 ||
      node.callee.type !== AST_NODE_TYPES.Identifier ||
      node.callee.name !== 'require' ||
      variableOf(this.scopeOf(node), 'require')?.defs.length
    )
      return undefined
    const argument = node.arguments[0]
    return argument.type === AST_NODE_TYPES.Literal &&
      typeof argument.value === 'string'
      ? sourceOf(argument.value)
      : undefined
  }
}
