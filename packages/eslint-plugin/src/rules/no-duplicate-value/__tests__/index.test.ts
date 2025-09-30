import { RuleTester } from '@typescript-eslint/rule-tester'

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
    ],
  })
})
