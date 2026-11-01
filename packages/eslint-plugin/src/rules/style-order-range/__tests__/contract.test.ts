import { readFileSync } from 'node:fs'

import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { styleOrderRange } from '../index'

const cases: { expression: string; valid: boolean }[] = JSON.parse(
  readFileSync(
    new URL(
      '../../../../../../test-fixtures/style-order.json',
      import.meta.url,
    ),
    'utf8',
  ),
)

describe('styleOrder matches the extractor contract', () => {
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const fixtures = cases.flatMap(({ expression, valid }) =>
    [
      `import { Box } from '@devup-ui/react'; <Box p={1} styleOrder={${expression}} />`,
      `import { css } from '@devup-ui/react'; css({ p: 1, styleOrder: ${expression} })`,
    ].map((code) => ({ code, valid, filename: 'contract.tsx' })),
  )
  ruleTester.run('style-order-range contract', styleOrderRange, {
    valid: fixtures
      .filter(({ valid }) => valid)
      .map(({ code, filename }) => ({ code, filename }))
      .concat(
        {
          code: `import { css } from '@devup-ui/react'; css({ styleOrder: <number>1 })`,
          filename: 'contract.ts',
        },
        {
          code: `import { css } from '@devup-ui/react'; css({ styleOrder: +(<number>1) })`,
          filename: 'contract.ts',
        },
      ),
    invalid: fixtures
      .filter(({ valid }) => !valid)
      .map(({ code, filename }) => ({
        code,
        filename,
        errors: [{ messageId: 'styleOrderRange' }],
      }))
      .concat([
        {
          code: `import { Box } from '@devup-ui/react'; <Box styleOrder />`,
          filename: 'contract.tsx',
          errors: [{ messageId: 'styleOrderRange' }],
        },
        {
          code: `import { css } from '@devup-ui/react'; css({ 'styleOrder': '100px' })`,
          filename: 'contract.tsx',
          errors: [{ messageId: 'styleOrderRange' }],
        },
      ]),
  })
})
