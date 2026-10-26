import { join } from 'node:path'

import { ESLintUtils, type TSESLint } from '@typescript-eslint/utils'
import { describe, expect, it } from 'bun:test'
import { Linter } from 'eslint'
import type { Type } from 'typescript'
import { parser } from 'typescript-eslint'

import { keyDomainsOverlap, keyDomainsOverlapExcept } from '../key-domain'

describe('key domain overlap', () => {
  const cases: readonly (readonly [string, string, boolean, string?])[] = [
    ['"a"', '"b"', false],
    ['never', 'string', false],
    ['string', 'never', false],
    ['"a"', '`a${string}b`', false],
    ['`a${string}`', '`${string}b`', true],
    ['`\\uD83D\\uDE00${string}`', '`\\uD83D${string}a`', true],
    ['`\\uD83D${string}a`', '`\\uD83D\\uDE00${string}`', true],
    [
      '`\\uD83D\\uDE00${string}`',
      '`\\uD83D${string}a`',
      false,
      '`\\uD83D\\uDE00${string}a`',
    ],
    ['`a${string}x`', '`a${string}y`', false],
    ['`@${string} (${string})`', '`@media ${string}`', true],
    ['`@${number} foo`', '`@media ${string}`', false],
    ['`a${string}` | `b${string}`', '`${string}b`', true],
    ['`${string}z`', '`a${string}` | `b${string}`', true],
    ['number', 'symbol', false],
    ['string', 'number', false],
    ['`@media ${string}`', '`@media ${string}`', true, '`@media ${number}`'],
    ['`@media ${string}`', '`@media ${string}`', false, '`@media ${string}`'],
    ['"bg"', 'string', true, '`${number}`'],
    ['string', '"bg"', true, '`${number}`'],
    ['`@media ${number}`', '`@media ${string}`', true, '"@media 0"'],
    ['`@media ${string}`', '`@media ${number}`', true, '"@media 0"'],
    ['`data-${number}`', '`@media ${string}`', false, '"@media 0"'],
    [
      '`@media ${number}px`',
      '`@media ${string}`',
      false,
      '`@media ${number}em`',
    ],
    ['`${number}px`', '`${number}em`', false, '"0"'],
    ['`a${string}`', '`${string}b`', false, '`a${string}b`'],
    ['`a${string}`', '`${string}b`', true, '"ab"'],
    ['`a${string}`', '`${string}b`', true, '`a${string}c${string}b`'],
    ['`a${string}a${string}`', '`${string}b`', false, '`a${string}b`'],
    ['`a${string}`', '`${string}b`', true, 'number | symbol'],
    ['""', 'string', true, '`a${string}`'],
    ['"a"', 'string', false, '`a${string}`'],
    ['"ab" | "ac"', '`a${string}`', false, '"ab" | "ac"'],
    ['"ab" | "ac"', '`a${string}`', true, '"ab"'],
    [
      '`@${string} (min-width: 1px)`',
      '`@media ${string}`',
      false,
      '`@media ${string}(min-width: 1px)`',
    ],
    [
      '`@${string} (min-width: 1px)`',
      '`@media ${string}`',
      true,
      '`@media ${string}(min-width: 1px)x`',
    ],
  ]
  for (const [left, right, expected, excluded = 'never'] of cases) {
    it(`returns ${expected} when intersecting ${left} with ${right} excluding ${excluded}`, () => {
      // Given two actual compiler key types, not reconstructed type objects.
      const root = join(import.meta.dir, 'fixtures/key-domain')
      const results: boolean[] = []
      const inspect: TSESLint.RuleModule<'unused', []> = {
        defaultOptions: [],
        meta: { schema: [], messages: { unused: 'unused' }, type: 'problem' },
        create(context) {
          const services = ESLintUtils.getParserServices(context)
          const types: Type[] = []
          return {
            VariableDeclarator(node) {
              types.push(services.getTypeAtLocation(node.id))
            },
            'Program:exit'() {
              const [a, b, mask] = types
              if (a && b && mask) {
                const checker = services.program.getTypeChecker()
                results.push(
                  mask === checker.getNeverType()
                    ? keyDomainsOverlap(checker, a, b)
                    : keyDomainsOverlapExcept(checker, [a, b], [mask]),
                )
              }
            },
          }
        },
      }
      // When querying overlap through the checker-backed helper.
      new Linter().verify(
        `declare const left: ${left}; declare const right: ${right}; declare const excluded: ${excluded}`,
        [
          {
            files: ['**/*.tsx'],
            languageOptions: {
              parser,
              parserOptions: { projectService: true, tsconfigRootDir: root },
            },
            plugins: { inspect: { rules: { domain: inspect } } },
            rules: { 'inspect/domain': 'error' },
          },
        ],
        { filename: join(root, 'case.tsx') },
      )
      // Then exactly one overlap answer matches the independently chosen contract.
      expect(results).toEqual([expected])
    }, 15000)
  }
})
