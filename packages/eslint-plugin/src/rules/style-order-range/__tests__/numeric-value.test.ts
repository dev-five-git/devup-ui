import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { styleOrderRange } from '../index'

describe('style-order-range unary coercion', () => {
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })

  ruleTester.run('style-order-range unary coercion', styleOrderRange, {
    valid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={+`2`} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({ styleOrder: +true })',
        filename: 'src/app/page.tsx',
      },
    ],
    invalid: [
      ...['+1n', '+/2/', '+`${2}`'].map((value) => ({
        code: `import { Box } from "@devup-ui/react";\n<Box styleOrder={${value}} />`,
        filename: 'src/app/page.tsx',
        output: null,
        errors: [{ messageId: 'styleOrderRange' }],
      })),
      ...['-1n', '-/2/', '-`${2}`'].map((value) => ({
        code: `import { css } from "@devup-ui/react";\ncss({ styleOrder: ${value} })`,
        filename: 'src/app/page.tsx',
        output: null,
        errors: [{ messageId: 'styleOrderRange' }],
      })),
    ],
  })
})
