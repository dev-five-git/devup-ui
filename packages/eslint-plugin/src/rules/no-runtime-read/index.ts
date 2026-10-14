import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

import {
  ImportStorage,
  STYLE_COMPONENTS,
  STYLE_FUNCTIONS,
} from '../../utils/import-storage'
import {
  componentName,
  isStyledFactory,
  isStyledReference,
} from '../../utils/style-position'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

type Variable = TSESLint.Scope.Variable
type Reference = TSESLint.Scope.Reference

/** What the build compiles away a name to, and so what it reads of that name */
type Kind = 'component' | 'function' | 'styled'

function kindOf(name: string): Kind | undefined {
  if (STYLE_COMPONENTS.has(name)) return 'component'
  if (STYLE_FUNCTIONS.has(name)) return 'function'
  return name === 'styled' ? 'styled' : undefined
}

type ChainLink =
  TSESTree.Identifier | TSESTree.MemberExpression | TSESTree.CallExpression

/** Whether the chain `styled.div`, `styled(tag)`, `.attrs()` ending at `node` is built into a component: called with rules or tagged with CSS text */
function buildsComponent(
  node: ChainLink,
  importStorage: ImportStorage,
): boolean {
  const parent = node.parent
  switch (parent.type) {
    case AST_NODE_TYPES.MemberExpression:
      return (
        parent.object === node &&
        !parent.computed &&
        buildsComponent(parent, importStorage)
      )
    case AST_NODE_TYPES.CallExpression:
      return (
        parent.callee === node &&
        (!isStyledFactory(parent, importStorage) ||
          buildsComponent(parent, importStorage))
      )
    case AST_NODE_TYPES.TaggedTemplateExpression:
      return parent.tag === node && node.type !== AST_NODE_TYPES.Identifier
    default:
      return false
  }
}
/** Whether `identifier` is in a type, which the build erases */
function inType(identifier: TSESTree.Node) {
  for (
    let current: TSESTree.Node | undefined = identifier;
    current;
    current = current.parent
  )
    if (current.type === AST_NODE_TYPES.TSTypeQuery) return true
  return false
}

export const noRuntimeRead = createRule({
  name: 'no-runtime-read',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      noRuntimeRead:
        '`{{name}}` is read at runtime, where it does not exist: the build compiles it only where it is called or rendered.',
    },
    type: 'problem',
    docs: {
      description:
        'Disallow reading what the build compiles away, anywhere but where it is called or rendered.',
    },
  },
  create(context) {
    const importStorage = new ImportStorage(context)
    return {
      ImportDeclaration(node) {
        importStorage.addImportByDeclaration(node)
      },
      'Program:exit'() {
        const scopes = context.sourceCode.scopeManager?.scopes ?? []
        const scope =
          scopes.find((candidate) => candidate.type === 'module') ?? scopes[0]
        if (!scope) return
        const pending: [Variable, Kind][] = []
        for (const [local, name] of importStorage.bindings()) {
          const kind = kindOf(name)
          const variable = scope.set.get(local)
          if (kind && variable) pending.push([variable, kind])
        }
        const seen = new Set<Variable>()
        for (let next = pending.pop(); next; next = pending.pop()) {
          const [variable, kind] = next
          if (seen.has(variable)) continue
          seen.add(variable)
          for (const reference of variable.references) {
            if (reference.init) continue
            const alias = aliasOf(reference, scope)
            if (alias) pending.push([alias, kind])
            else if (!reads(reference, kind))
              context.report({
                node: reference.identifier,
                messageId: 'noRuntimeRead',
                data: { name: reference.identifier.name },
              })
          }
        }
      },
    }

    /** The module-level binding that only holds what the build compiles away, which the build removes (`const newCss = css`) */
    function aliasOf(
      reference: Reference,
      scope: TSESLint.Scope.Scope,
    ): Variable | undefined {
      const parent = reference.identifier.parent
      return parent.type === AST_NODE_TYPES.VariableDeclarator &&
        parent.init === reference.identifier &&
        parent.id.type === AST_NODE_TYPES.Identifier &&
        parent.parent.parent.type === AST_NODE_TYPES.Program
        ? scope.set.get(parent.id.name)
        : undefined
    }

    /** Whether the build compiles the read, or erases it */
    function reads(reference: Reference, kind: Kind): boolean {
      const identifier = reference.identifier
      if (inType(identifier)) return true
      const parent = identifier.parent
      switch (kind) {
        case 'function':
          return (
            (parent.type === AST_NODE_TYPES.CallExpression &&
              parent.callee === identifier) ||
            (parent.type === AST_NODE_TYPES.TaggedTemplateExpression &&
              parent.tag === identifier)
          )
        case 'styled':
          return (
            identifier.type === AST_NODE_TYPES.Identifier &&
            buildsComponent(identifier, importStorage)
          )
        case 'component':
          return rendersOrBases(identifier, parent)
      }
    }

    function rendersOrBases(
      identifier: TSESTree.Node,
      parent: TSESTree.Node,
    ): boolean {
      switch (parent.type) {
        case AST_NODE_TYPES.JSXOpeningElement:
        case AST_NODE_TYPES.JSXClosingElement:
          return parent.name === identifier
        case AST_NODE_TYPES.JSXExpressionContainer: {
          const attribute = parent.parent
          return (
            attribute.type === AST_NODE_TYPES.JSXAttribute &&
            attribute.name.type === AST_NODE_TYPES.JSXIdentifier &&
            attribute.name.name === 'as' &&
            STYLE_COMPONENTS.has(
              componentName(attribute.parent.name, importStorage) ?? '',
            )
          )
        }
        case AST_NODE_TYPES.CallExpression:
          return (
            parent.arguments[0] === identifier &&
            isStyledReference(parent.callee, importStorage) &&
            (!isStyledFactory(parent, importStorage) ||
              buildsComponent(parent, importStorage))
          )
        default:
          return false
      }
    }
  },
})
