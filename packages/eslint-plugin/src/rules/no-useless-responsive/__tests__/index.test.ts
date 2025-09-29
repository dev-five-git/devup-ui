import { RuleTester } from '@typescript-eslint/rule-tester'

import { noUselessResponsive } from '../index'

describe('no-useless-responsive rule', () => {
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
  ruleTester.run('no-useless-responsive rule', noUselessResponsive, {
    valid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={1} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={"1"} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "other-package";\n<Box w={[1][0]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({w: 1})',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({w: "1"})',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "other-package";\ncss({w: [1][0]})',
        filename: 'src/app/page.tsx',
      },
    ],
    invalid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1]} />',
        output: 'import { Box } from "@devup-ui/react";\n<Box w={1} />',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
    ],
  })
})
