import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'
import { describe, expect, it } from 'bun:test'

import { ImportStorage } from '../import-storage'

describe('ImportStorage', () => {
  describe('addImportByDeclaration', () => {
    it('should ignore non-devup-ui imports', () => {
      const storage = new ImportStorage()
      const node = {
        type: AST_NODE_TYPES.ImportDeclaration,
        source: {
          type: AST_NODE_TYPES.Literal,
          value: 'other-package',
          raw: '"other-package"',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        specifiers: [],
        importKind: 'value',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.ImportDeclaration

      storage.addImportByDeclaration(node)
      // Should not throw and should not add any imports
    })

    it('should handle ImportSpecifier with Literal imported name', () => {
      const storage = new ImportStorage()
      const node = {
        type: AST_NODE_TYPES.ImportDeclaration,
        source: {
          type: AST_NODE_TYPES.Literal,
          value: '@devup-ui/react',
          raw: '"@devup-ui/react"',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        specifiers: [
          {
            type: AST_NODE_TYPES.ImportSpecifier,
            local: {
              type: AST_NODE_TYPES.Identifier,
              name: 'B',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            imported: {
              type: AST_NODE_TYPES.Literal,
              value: 'Box',
              raw: '"Box"',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            importKind: 'value',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
        ],
        importKind: 'value',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.ImportDeclaration

      storage.addImportByDeclaration(node)
      // Check that 'B' is mapped to 'Box'
      const jsxNode = {
        type: AST_NODE_TYPES.JSXOpeningElement,
        name: {
          type: AST_NODE_TYPES.JSXIdentifier,
          name: 'B',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        attributes: [],
        selfClosing: true,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.JSXOpeningElement
      expect(storage.checkContextType(jsxNode)).toBe('COMPONENT')
    })
  })

  describe('addImport', () => {
    it('should add import directly', () => {
      const storage = new ImportStorage()
      storage.addImport('MyBox', 'Box')

      const jsxNode = {
        type: AST_NODE_TYPES.JSXOpeningElement,
        name: {
          type: AST_NODE_TYPES.JSXIdentifier,
          name: 'MyBox',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        attributes: [],
        selfClosing: true,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.JSXOpeningElement
      expect(storage.checkContextType(jsxNode)).toBe('COMPONENT')
    })
  })

  describe('checkContextType', () => {
    it('should return undefined for unsupported node types', () => {
      const storage = new ImportStorage()
      const node = {
        type: AST_NODE_TYPES.Identifier,
        name: 'test',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.Node
      expect(storage.checkContextType(node)).toBeUndefined()
    })

    it('should return undefined for non-devup JSX component', () => {
      const storage = new ImportStorage()
      const jsxNode = {
        type: AST_NODE_TYPES.JSXOpeningElement,
        name: {
          type: AST_NODE_TYPES.JSXIdentifier,
          name: 'SomeOtherComponent',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        attributes: [],
        selfClosing: true,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.JSXOpeningElement
      expect(storage.checkContextType(jsxNode)).toBeUndefined()
    })

    it('should return undefined for non-devup call expression', () => {
      const storage = new ImportStorage()
      const callNode = {
        type: AST_NODE_TYPES.CallExpression,
        callee: {
          type: AST_NODE_TYPES.Identifier,
          name: 'someFunction',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        arguments: [],
        optional: false,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.CallExpression
      expect(storage.checkContextType(callNode)).toBeUndefined()
    })

    it('should return UTIL for devup call expression', () => {
      const storage = new ImportStorage()
      storage.addImport('css', 'css')

      const callNode = {
        type: AST_NODE_TYPES.CallExpression,
        callee: {
          type: AST_NODE_TYPES.Identifier,
          name: 'css',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        arguments: [],
        optional: false,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.CallExpression
      expect(storage.checkContextType(callNode)).toBe('UTIL')
    })

    it('should return UTIL for member expression call on import object', () => {
      const storage = new ImportStorage()
      // Simulate: import devup from "@devup-ui/react"; devup.css()
      const node = {
        type: AST_NODE_TYPES.ImportDeclaration,
        source: {
          type: AST_NODE_TYPES.Literal,
          value: '@devup-ui/react',
          raw: '"@devup-ui/react"',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        specifiers: [
          {
            type: AST_NODE_TYPES.ImportDefaultSpecifier,
            local: {
              type: AST_NODE_TYPES.Identifier,
              name: 'devup',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
        ],
        importKind: 'value',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.ImportDeclaration
      storage.addImportByDeclaration(node)

      const callNode = {
        type: AST_NODE_TYPES.CallExpression,
        callee: {
          type: AST_NODE_TYPES.MemberExpression,
          object: {
            type: AST_NODE_TYPES.Identifier,
            name: 'devup',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.Identifier,
            name: 'css',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          computed: false,
          optional: false,
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        arguments: [],
        optional: false,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.CallExpression
      expect(storage.checkContextType(callNode)).toBe('UTIL')
    })

    it('should return COMPONENT for JSX member expression on import object', () => {
      const storage = new ImportStorage()
      // Simulate: import * as devup from "@devup-ui/react"; <devup.Box />
      const node = {
        type: AST_NODE_TYPES.ImportDeclaration,
        source: {
          type: AST_NODE_TYPES.Literal,
          value: '@devup-ui/react',
          raw: '"@devup-ui/react"',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        specifiers: [
          {
            type: AST_NODE_TYPES.ImportNamespaceSpecifier,
            local: {
              type: AST_NODE_TYPES.Identifier,
              name: 'devup',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
        ],
        importKind: 'value',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.ImportDeclaration
      storage.addImportByDeclaration(node)

      const jsxNode = {
        type: AST_NODE_TYPES.JSXOpeningElement,
        name: {
          type: AST_NODE_TYPES.JSXMemberExpression,
          object: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'devup',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'Box',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        attributes: [],
        selfClosing: true,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.JSXOpeningElement
      expect(storage.checkContextType(jsxNode)).toBe('COMPONENT')
    })

    it('should return undefined for non-devup member expression call', () => {
      const storage = new ImportStorage()
      // Simulate: import other from "other-package"; other.css()
      const callNode = {
        type: AST_NODE_TYPES.CallExpression,
        callee: {
          type: AST_NODE_TYPES.MemberExpression,
          object: {
            type: AST_NODE_TYPES.Identifier,
            name: 'other',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.Identifier,
            name: 'css',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          computed: false,
          optional: false,
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        arguments: [],
        optional: false,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.CallExpression
      expect(storage.checkContextType(callNode)).toBeUndefined()
    })

    it('should return undefined for member expression with non-identifier object', () => {
      const storage = new ImportStorage()

      const callNode = {
        type: AST_NODE_TYPES.CallExpression,
        callee: {
          type: AST_NODE_TYPES.MemberExpression,
          object: {
            type: AST_NODE_TYPES.CallExpression,
            callee: {
              type: AST_NODE_TYPES.Identifier,
              name: 'getModule',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            arguments: [],
            optional: false,
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.Identifier,
            name: 'css',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          computed: false,
          optional: false,
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        arguments: [],
        optional: false,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.CallExpression
      expect(storage.checkContextType(callNode)).toBeUndefined()
    })

    it('should return undefined for member expression with computed property', () => {
      const storage = new ImportStorage()
      const node = {
        type: AST_NODE_TYPES.ImportDeclaration,
        source: {
          type: AST_NODE_TYPES.Literal,
          value: '@devup-ui/react',
          raw: '"@devup-ui/react"',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        specifiers: [
          {
            type: AST_NODE_TYPES.ImportDefaultSpecifier,
            local: {
              type: AST_NODE_TYPES.Identifier,
              name: 'devup',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
        ],
        importKind: 'value',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.ImportDeclaration
      storage.addImportByDeclaration(node)

      const callNode = {
        type: AST_NODE_TYPES.CallExpression,
        callee: {
          type: AST_NODE_TYPES.MemberExpression,
          object: {
            type: AST_NODE_TYPES.Identifier,
            name: 'devup',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.Literal,
            value: 'css',
            raw: '"css"',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          computed: true,
          optional: false,
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        arguments: [],
        optional: false,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.CallExpression
      expect(storage.checkContextType(callNode)).toBeUndefined()
    })

    it('should return undefined for non-devup property on member expression', () => {
      const storage = new ImportStorage()
      const node = {
        type: AST_NODE_TYPES.ImportDeclaration,
        source: {
          type: AST_NODE_TYPES.Literal,
          value: '@devup-ui/react',
          raw: '"@devup-ui/react"',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        specifiers: [
          {
            type: AST_NODE_TYPES.ImportDefaultSpecifier,
            local: {
              type: AST_NODE_TYPES.Identifier,
              name: 'devup',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
        ],
        importKind: 'value',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.ImportDeclaration
      storage.addImportByDeclaration(node)

      const callNode = {
        type: AST_NODE_TYPES.CallExpression,
        callee: {
          type: AST_NODE_TYPES.MemberExpression,
          object: {
            type: AST_NODE_TYPES.Identifier,
            name: 'devup',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.Identifier,
            name: 'unknownMethod',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          computed: false,
          optional: false,
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        arguments: [],
        optional: false,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.CallExpression
      expect(storage.checkContextType(callNode)).toBeUndefined()
    })

    it('should return undefined for JSX member expression with non-JSXIdentifier object', () => {
      const storage = new ImportStorage()
      const node = {
        type: AST_NODE_TYPES.ImportDeclaration,
        source: {
          type: AST_NODE_TYPES.Literal,
          value: '@devup-ui/react',
          raw: '"@devup-ui/react"',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        specifiers: [
          {
            type: AST_NODE_TYPES.ImportNamespaceSpecifier,
            local: {
              type: AST_NODE_TYPES.Identifier,
              name: 'devup',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
        ],
        importKind: 'value',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.ImportDeclaration
      storage.addImportByDeclaration(node)

      // Nested member expression: devup.components.Box
      const jsxNode = {
        type: AST_NODE_TYPES.JSXOpeningElement,
        name: {
          type: AST_NODE_TYPES.JSXMemberExpression,
          object: {
            type: AST_NODE_TYPES.JSXMemberExpression,
            object: {
              type: AST_NODE_TYPES.JSXIdentifier,
              name: 'devup',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            property: {
              type: AST_NODE_TYPES.JSXIdentifier,
              name: 'components',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'Box',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        attributes: [],
        selfClosing: true,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.JSXOpeningElement
      expect(storage.checkContextType(jsxNode)).toBeUndefined()
    })

    it('should return undefined for JSX member expression with unknown property', () => {
      const storage = new ImportStorage()
      const node = {
        type: AST_NODE_TYPES.ImportDeclaration,
        source: {
          type: AST_NODE_TYPES.Literal,
          value: '@devup-ui/react',
          raw: '"@devup-ui/react"',
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        specifiers: [
          {
            type: AST_NODE_TYPES.ImportNamespaceSpecifier,
            local: {
              type: AST_NODE_TYPES.Identifier,
              name: 'devup',
              range: [0, 0],
              loc: {
                start: { line: 0, column: 0 },
                end: { line: 0, column: 0 },
              },
            },
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
        ],
        importKind: 'value',
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.ImportDeclaration
      storage.addImportByDeclaration(node)

      const jsxNode = {
        type: AST_NODE_TYPES.JSXOpeningElement,
        name: {
          type: AST_NODE_TYPES.JSXMemberExpression,
          object: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'devup',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'UnknownComponent',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        attributes: [],
        selfClosing: true,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.JSXOpeningElement
      expect(storage.checkContextType(jsxNode)).toBeUndefined()
    })

    it('should return undefined for JSX member expression with non-registered object', () => {
      const storage = new ImportStorage()

      const jsxNode = {
        type: AST_NODE_TYPES.JSXOpeningElement,
        name: {
          type: AST_NODE_TYPES.JSXMemberExpression,
          object: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'unknown',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          property: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'Box',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        attributes: [],
        selfClosing: true,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.JSXOpeningElement
      expect(storage.checkContextType(jsxNode)).toBeUndefined()
    })

    it('should return undefined for JSXNamespacedName', () => {
      const storage = new ImportStorage()
      storage.addImport('Box', 'Box')

      const jsxNode = {
        type: AST_NODE_TYPES.JSXOpeningElement,
        name: {
          type: AST_NODE_TYPES.JSXNamespacedName,
          namespace: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'ns',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          name: {
            type: AST_NODE_TYPES.JSXIdentifier,
            name: 'Box',
            range: [0, 0],
            loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
          },
          range: [0, 0],
          loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
        },
        attributes: [],
        selfClosing: true,
        range: [0, 0],
        loc: { start: { line: 0, column: 0 }, end: { line: 0, column: 0 } },
      } as unknown as TSESTree.JSXOpeningElement
      expect(storage.checkContextType(jsxNode)).toBeUndefined()
    })
  })
})
