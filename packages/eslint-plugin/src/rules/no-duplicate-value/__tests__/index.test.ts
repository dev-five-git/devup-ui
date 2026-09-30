import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { noDuplicateValue } from '../index'

describe('no-duplicate-value rule', () => {
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
  ruleTester.run('no-duplicate-value rule', noDuplicateValue, {
    valid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, 3]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, 3][1]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "other-package";\n<Box w={[1, null, 2, 3]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, null, null, 2, 3]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[null, null, null, 3]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "other-package";\ncss()',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[call(), null, null, 3]} />',
        filename: 'src/app/page.tsx',
      },
      ...[
        '<Box data-values={[5, 5]} />',
        '<Box aria-x={[5, 5]} />',
        '<Box onPick={() => pick([5, 5])} />',
        '<Box props={{ items: [5, 5] }} />',
        '<Box styleVars={{ a: [5, 5] }} />',
        '<Box w={pick([5, 5])} />',
        '<Box xlink:href={[5, 5]} />',
        '<ThemeScript x={[5, 5]} />',
        '<Devup.Other w={[5, 5]} />',
        'getTheme([5, 5])',
        'Devup.getTheme([5, 5])',
        'globalCss({ imports: ["a.css", "a.css"] })',
        'css({ [[5, 5]]: 1 })',
      ].map((use) => ({
        code: `import { Box, ThemeScript, getTheme, globalCss, css } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\n${use}`,
        filename: 'src/app/page.tsx',
      })),
    ],
    invalid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 1, 1]} />',
        output:
          'import { Box } from "@devup-ui/react";\n<Box w={[1, null, null]} />',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'duplicateValue',
          },
          {
            messageId: 'duplicateValue',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, 2, 3]} />',
        output:
          'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, null, 3]} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'duplicateValue',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({w: [1, 2, 2, 2, 3]})',
        output:
          'import { css } from "@devup-ui/react";\ncss({w: [1, 2, null, null, 3]})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'duplicateValue',
          },
          {
            messageId: 'duplicateValue',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({w: [1, `2`, 2, "2", 3]})',
        output:
          'import { css } from "@devup-ui/react";\ncss({w: [1, `2`, null, null, 3]})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'duplicateValue',
          },
          {
            messageId: 'duplicateValue',
          },
        ],
      },
      ...[
        ['<Box w={on ? [1, 1] : 2} />', '<Box w={on ? [1, null] : 2} />'],
        ['<Box w={on && [1, 1]} />', '<Box w={on && [1, null]} />'],
        ['<Box {...{ w: [1, 1] }} />', '<Box {...{ w: [1, null] }} />'],
        ['<Box w={[1, 1] as any} />', '<Box w={[1, null] as any} />'],
        [
          '<Box w={[1, 1] satisfies number[]} />',
          '<Box w={[1, null] satisfies number[]} />',
        ],
        ['<Box w={[1, 1]!} />', '<Box w={[1, null]!} />'],
        ['<Box _hover={{ w: [1, 1] }} />', '<Box _hover={{ w: [1, null] }} />'],
        ['<Devup.Box w={[1, 1]} />', '<Devup.Box w={[1, null]} />'],
        ['Devup.css({ w: [1, 1] })', 'Devup.css({ w: [1, null] })'],
        ['css(base, { w: [1, 1] })', 'css(base, { w: [1, null] })'],
        ['css({ ...{ w: [1, 1] } })', 'css({ ...{ w: [1, null] } })'],
      ].map(([use, fixed]) => ({
        code: `import { Box, css } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\n${use}`,
        output: `import { Box, css } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\n${fixed}`,
        filename: 'src/app/page.tsx',
        errors: [{ messageId: 'duplicateValue' as const }],
      })),
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box icon={<Box w={[1, 1]} />} m={[2, 2]} />',
        output:
          'import { Box } from "@devup-ui/react";\n<Box icon={<Box w={[1, null]} />} m={[2, null]} />',
        filename: 'src/app/page.tsx',
        errors: [
          { messageId: 'duplicateValue' },
          { messageId: 'duplicateValue' },
        ],
      },
    ],
  })
})
