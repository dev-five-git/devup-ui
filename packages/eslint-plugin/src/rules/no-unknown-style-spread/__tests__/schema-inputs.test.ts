import { join } from 'node:path'

import { ESLintUtils, type TSESLint } from '@typescript-eslint/utils'
import { describe, expect, it } from 'bun:test'
import { Linter } from 'eslint'
import { parser } from 'typescript-eslint'

import { styleCapability, StyleSchemaError } from '../style-capability'

describe('invalid style schema inputs', () => {
  const root = join(import.meta.dir, 'fixtures')
  const filename = join(root, 'case.tsx')
  const inspect: TSESLint.RuleModule<'unused', []> = {
    defaultOptions: [],
    meta: { schema: [], messages: { unused: 'unused' }, type: 'problem' },
    create(context) {
      const services = ESLintUtils.getParserServices(context)
      return {
        VariableDeclarator(node) {
          styleCapability(
            services.program.getTypeChecker(),
            services.getTypeAtLocation(node.id),
            context.filename,
          )
        },
      }
    },
  }
  for (const type of [
    'string',
    '() => React.ReactElement',
    '(props: any) => React.ReactElement',
    '(props: { bg?: string }) => string',
    '(props: { bg?: string }) => Local.Element',
    '(props: { bg?: string }) => Missing.Element',
  ]) {
    it(`rejects an invalid schema when the checker resolves ${type}`, () => {
      // Given real checker types with an invalid callable/props/element contract.
      const code = `import type * as React from 'react'; import type * as Missing from './no-intrinsics'; declare namespace Local { interface Element {} }; declare const candidate: ${type}`
      const configs = [
        {
          files: ['**/*.tsx'],
          languageOptions: {
            parser,
            parserOptions: { projectService: true, tsconfigRootDir: root },
          },
          plugins: { inspect: { rules: { schema: inspect } } },
          rules: { 'inspect/schema': 'error' as const },
        },
      ]
      // When deriving the schema, then a typed resolution error must propagate.
      expect(() => new Linter().verify(code, configs, { filename })).toThrow(
        StyleSchemaError,
      )
    }, 30000)
  }
})
