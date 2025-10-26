import { RuleTester } from '@typescript-eslint/rule-tester'

import { noUselessTailingNulls } from '../index'

describe('no-useless-tailing-nulls rule', () => {
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
    ],
    invalid: [
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
