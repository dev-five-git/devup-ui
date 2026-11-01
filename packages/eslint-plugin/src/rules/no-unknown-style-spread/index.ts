import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'
import type { Symbol } from 'typescript'

import { ImportStorage, STYLE_COMPONENTS } from '../../utils/import-storage'
import { componentName } from '../../utils/style-position'
import { findBinding, runtimeSource } from './runtime-source'
import { styleCapability, StyleSchemaError } from './style-capability'

const createRule = ESLintUtils.RuleCreator<{ requiresTypeChecking: boolean }>(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

export const noUnknownStyleSpread = createRule({
  name: 'no-unknown-style-spread',
  defaultOptions: [],
  meta: {
    type: 'problem',
    schema: [],
    docs: {
      description:
        'Disallow provably runtime spreads whose types can carry Devup styles.',
      requiresTypeChecking: true,
    },
    messages: {
      unknownStyleSpread:
        'This runtime spread can carry Devup style props. Constrain the type or pass style props explicitly; use build-time-known composition or forward DOM-only props separately.',
    },
  },
  create(context) {
    const services = ESLintUtils.getParserServices(context)
    const checker = services.program.getTypeChecker()
    const storage = new ImportStorage(context)
    const unknown = runtimeSource(context.sourceCode)
    const spreads: TSESTree.JSXSpreadAttribute[] = []
    const capabilities = new Map<Symbol, ReturnType<typeof styleCapability>>()
    return {
      ImportDeclaration(node) {
        storage.addImportByDeclaration(node)
      },
      JSXSpreadAttribute(node) {
        spreads.push(node)
      },
      'Program:exit'() {
        for (const node of spreads) {
          const name = node.parent.name
          const exportedName = componentName(name, storage) ?? ''
          if (!STYLE_COMPONENTS.has(exportedName) || !unknown(node.argument))
            continue
          const root =
            name.type === AST_NODE_TYPES.JSXMemberExpression
              ? name.object
              : name
          if (root.type !== AST_NODE_TYPES.JSXIdentifier) continue
          const variable = findBinding(context.sourceCode, root, root.name)
          const definition = variable?.defs[0]
          if (definition?.type !== 'ImportBinding') continue
          const declaration = definition.parent
          if (declaration.type !== AST_NODE_TYPES.ImportDeclaration) continue
          if (
            declaration.importKind === 'type' ||
            (definition.node.type === AST_NODE_TYPES.ImportSpecifier &&
              definition.node.importKind === 'type')
          )
            continue
          const module = checker.getSymbolAtLocation(
            services.esTreeNodeToTSNodeMap.get(declaration.source),
          )
          const exported =
            module &&
            checker
              .getExportsOfModule(module)
              .find((symbol) => symbol.name === exportedName)
          if (!exported) throw new StyleSchemaError(context.filename)
          let capable = capabilities.get(exported)
          if (!capable) {
            capable = styleCapability(
              checker,
              checker.getTypeOfSymbolAtLocation(
                exported,
                services.esTreeNodeToTSNodeMap.get(declaration),
              ),
              context.filename,
            )
            capabilities.set(exported, capable)
          }
          if (capable(services.getTypeAtLocation(node.argument))) {
            context.report({ node, messageId: 'unknownStyleSpread' })
          }
        }
      },
    }
  },
})
