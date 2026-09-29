import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { noTypographyTokenPrefix } from '../index'

describe('no-typography-token-prefix rule', () => {
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const imports = 'import { Box, css } from "@devup-ui/react";\n'
  ruleTester.run('no-typography-token-prefix rule', noTypographyTokenPrefix, {
    valid: [
      { code: `${imports}<Box typography="heading" />` },
      { code: `${imports}<Box color="$primary" />` },
      { code: `${imports}<Box typography={size} />` },
      { code: `${imports}<Box typography={a === '$x' ? 'b' : 'c'} />` },
      { code: `${imports}<Box typography={['$x'].length} />` },
      { code: `${imports}<Box data-typography="$x" />` },
      { code: `${imports}css({ color: '$primary', ['typography']: 1 })` },
      { code: `${imports}css({ '$heading': 1 })` },
      { code: `${imports}css({ typography: 1 })` },
      { code: `import { Box } from "other";\n<Box typography="$heading" />` },
      { code: `const a = { typography: '$heading' }` },
    ],
    invalid: [
      {
        code: `${imports}<Box typography="$heading" />`,
        output: `${imports}<Box typography="heading" />`,
        errors: [
          { messageId: 'noTypographyTokenPrefix', data: { name: 'heading' } },
        ],
      },
      {
        code: `${imports}<Box typography={['$a', null, cond ? '$b' : cond2 && 'c']} />`,
        output: `${imports}<Box typography={['a', null, cond ? 'b' : cond2 && 'c']} />`,
        errors: [
          { messageId: 'noTypographyTokenPrefix' },
          { messageId: 'noTypographyTokenPrefix' },
        ],
      },
      {
        code: `${imports}css({ _hover: { typography: '$title' } })`,
        output: `${imports}css({ _hover: { typography: 'title' } })`,
        errors: [{ messageId: 'noTypographyTokenPrefix' }],
      },
      {
        code: `${imports}<Box _hover={{ 'typography': "$title" }} />`,
        output: `${imports}<Box _hover={{ 'typography': "title" }} />`,
        errors: [{ messageId: 'noTypographyTokenPrefix' }],
      },
    ],
  })
})
