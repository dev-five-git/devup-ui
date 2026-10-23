import { RuleTester } from '@typescript-eslint/rule-tester'
import { TSESLint } from '@typescript-eslint/utils'
import { describe, expect, it } from 'bun:test'
import { parser } from 'typescript-eslint'

import { noUselessTailingNulls } from '../index'

describe('no-useless-tailing-nulls rule', () => {
  const boundaryCases = [
    ['[0,(1),,null]', '[0,(1)]'],
    ['[0,((1)),,null]', '[0,((1))]'],
    ['[0,(1),null]', '[0,(1)]'],
    ['[0,(1),,(null)]', '[0,(1)]'],
    ['[(null)]', '[]'],
    ['[1,null]', '[1]'],
    ['[0,((1)),,((null))]', '[0,((1))]'],
    ['[((null)),(null)]', '[]'],
    [
      '[0,( /* value */ 1 /* inner */ ), /* remove */ , (null)]',
      '[0,( /* value */ 1 /* inner */ )]',
    ],
    ['[1 /* remove */, /* remove */ null /* keep */]', '[1 /* keep */]'],
    ['[/* keep */ (null),null]', '[/* keep */ ]'],
    ['[, /* remove */ (null)]', '[]'],
    ['[0,(1),null,]', '[0,(1),]'],
    ['[0,((1)),,((null)),]', '[0,((1)),]'],
    ['[(null),]', '[,]'],
    ['[null,]', '[,]'],
    ['[1,null, /* keep */]', '[1, /* keep */]'],
    ['[1,null /* keep */,]', '[1 /* keep */,]'],
    [
      '[0,(1), /* remove */ null /* keep */, /* tail */]',
      '[0,(1) /* keep */, /* tail */]',
    ],
    ['[/* keep */ ((null)) /* tail */]', '[/* keep */  /* tail */]'],
  ] as const
  const literalKindCases = [
    ['[/(/,null]', '[/(/]'],
    ['[/a/gg,null]', '[/a/gg]'],
    ['[/a/z,null]', '[/a/z]'],
    ['[/a/gg,,null]', '[/a/gg]'],
    ['[/a/g,null]', '[/a/g]'],
    ['[/a/g,,null]', '[/a/g]'],
    ['[(/(/),,(null)]', '[(/(/)]'],
    ['[1n,null]', '[1n]'],
    ['[0xFFn,,null]', '[0xFFn]'],
    ['[1n,null,,(null),]', '[1n,]'],
  ] as const
  const literalKindControls = [
    '[/(/]',
    '[/a/gg]',
    '[/a/z]',
    '[1,null,/(/]',
    '[/a/g]',
    '[1n]',
    '[0xFFn]',
    '[null,1n]',
  ] as const
  const literalKindValidTests = literalKindControls.flatMap((value) =>
    [false, true].map((jsx) => ({
      code: `import {css,Box} from '@devup-ui/react';\n${jsx ? `<Box w={${value}} />` : `css({w:${value}})`}`,
      filename: 'src/app/page.tsx',
    })),
  )
  const boundaryTests = [...boundaryCases, ...literalKindCases].flatMap(
    ([value, fixed]) =>
      [false, true].map((jsx) => ({
        code: `import {css,Box} from '@devup-ui/react';\n${jsx ? `<Box w={${value}} />` : `css({w:${value}})`}`,
        output: `import {css,Box} from '@devup-ui/react';\n${jsx ? `<Box w={${fixed}} />` : `css({w:${fixed}})`}`,
        filename: 'src/app/page.tsx',
        errors: [{ messageId: 'uselessTailingNulls' as const }],
      })),
  )
  it.each([
    ...boundaryTests,
    ...literalKindValidTests.map((test) => ({ ...test, output: test.code })),
  ])(
    'preserves authored syntax and parses when fixing $code',
    ({ code, output }) => {
      const linter = new TSESLint.Linter()
      const config = {
        files: ['**/*.tsx'],
        languageOptions: {
          parser,
          parserOptions: { ecmaFeatures: { jsx: true } },
        },
        plugins: { devup: { rules: { trailing: noUselessTailingNulls } } },
        rules: { 'devup/trailing': 'error' },
      } satisfies TSESLint.FlatConfig.Config

      const result = linter.verifyAndFix(code, config, {
        filename: 'src/app/page.tsx',
      })

      expect(result.output).toBe(output)
      expect(
        linter.verify(
          result.output,
          { ...config, rules: {} },
          {
            filename: 'src/app/page.tsx',
          },
        ),
      ).toEqual([])
      expect(result.messages).toEqual([])
    },
  )
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
  })
  ruleTester.run('no-useless-tailing-nulls rule', noUselessTailingNulls, {
    valid: [
      ...literalKindValidTests,
      // Real parser holes are absent elements, not Literal null nodes.
      ...[
        '[]',
        '[,]',
        '[1,null,,]',
        '[null,,]',
        '[,,]',
        '[1,,]',
        '[1,,2]',
        '[1, null, 2]',
      ].map((value) => ({
        code: `import { css } from "@devup-ui/react";\ncss({w: ${value}})`,
        filename: 'src/app/page.tsx',
      })),
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, null, 3]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, null, null][1]} />',
        filename: 'src/app/page.tsx',
      },
      {
        // normal case
        code: 'import { Box } from "other-package";\n<Box w={[1, 2,]} />',
        filename: 'src/app/page.tsx',
      },
      {
        // normal case
        code: 'css({ w: [1, 2, null] })',
        filename: 'src/app/page.tsx',
      },
      ...[
        '<Box data-rows={[3, null, null]} />',
        '<Box props={{ rows: [3, null] }} />',
        '<Box w={pick([3, null])} />',
      ].map((use) => ({
        code: `import { Box } from "@devup-ui/react";\n${use}`,
        filename: 'src/app/page.tsx',
      })),
    ],
    invalid: [
      ...boundaryTests,
      ...[
        ['[null]', '[]'],
        ['[null,null]', '[]'],
        ['[1,null]', '[1]'],
        ['[1,,null]', '[1]'],
        ['[,1,null]', '[,1]'],
        ['[1,,2,null]', '[1,,2]'],
        ['[1,,null,null]', '[1]'],
        ['[,null]', '[]'],
        ['[,,null,null]', '[]'],
        ['[/* keep */ null,null]', '[/* keep */ ]'],
        ['[1,null,,null]', '[1]'],
        ['[null,1,,null]', '[null,1]'],
        ['[value,,null]', '[value]'],
        ['[...values,,null]', '[...values]'],
      ].map(([value, fixed]) => ({
        code: `import { css } from "@devup-ui/react";\ncss({w: ${value}})`,
        output: `import { css } from "@devup-ui/react";\ncss({w: ${fixed}})`,
        filename: 'src/app/page.tsx',
        errors: [{ messageId: 'uselessTailingNulls' as const }],
      })),
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box icon={<Box w={[1, null]} />} m={[2, null]} />',
        output:
          'import { Box } from "@devup-ui/react";\n<Box icon={<Box w={[1]} />} m={[2]} />',
        filename: 'src/app/page.tsx',
        errors: [
          { messageId: 'uselessTailingNulls' },
          { messageId: 'uselessTailingNulls' },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, null]} />',
        output: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2]} />',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessTailingNulls',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, null, null]} />',
        output: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2]} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'uselessTailingNulls',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({w: [1, 2, null, null]})',
        output: 'import { css } from "@devup-ui/react";\ncss({w: [1, 2]})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'uselessTailingNulls',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[null, null, null, null]} />',
        output: 'import { Box } from "@devup-ui/react";\n<Box w={[]} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'uselessTailingNulls',
          },
        ],
      },
    ],
  })
})
