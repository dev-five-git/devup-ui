import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { noDuplicateValue } from '../../rules/no-duplicate-value'
import { noUselessResponsive } from '../../rules/no-useless-responsive'
import { noUselessTailingNulls } from '../../rules/no-useless-tailing-nulls'

describe('composition arrays are not responsive values', () => {
  const tester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const valid = [
    `import { css } from '@emotion/react'; css(base, 'extra', [{ p: 1 }], null)`,
    `import { css } from '@emotion/react'; export const result = css(base, 'extra', [{ p: 1 }], null)`,
    `import { style } from '@vanilla-extract/css'; export const result = style([[{ color: 'red' }], false && { margin: 0 }])`,
    `import { css } from '@devup-ui/react'; css(['base', 'base', null])`,
    `import { css } from '@devup-ui/react'; const parts = ['base', 'base', null]; css(parts)`,
    `import { css } from '@devup-ui/react'; const parts = ['base']; const alias = parts; css(alias)`,
    `import { Global } from '@emotion/react'; <Global styles={[{ body: { color: 'red' } }]} />`,
    `import styled from 'styled-components'; styled.div([{ color: 'red' }])`,
    `import { css, Box } from '@devup-ui/react'; const parts = ['base']; css(parts); <Box color={parts} />`,
    `import { Box } from '@devup-ui/react'; <Box p={[[1, 1, null], 2]} />`,
  ].map((code) => ({ code, filename: 'composition.tsx' }))
  tester.run('no-useless-responsive composition', noUselessResponsive, {
    valid,
    invalid: [
      {
        code: `import { css } from '@devup-ui/react'; css([{ p: [1] }])`,
        output: `import { css } from '@devup-ui/react'; css([{ p: 1 }])`,
        filename: 'composition.tsx',
        errors: [{ messageId: 'uselessResponsive' }],
      },
      {
        code: `import { Box } from '@devup-ui/react'; const value = [1]; <Box p={value} />`,
        output: `import { Box } from '@devup-ui/react'; const value = 1; <Box p={value} />`,
        filename: 'composition.tsx',
        errors: [{ messageId: 'uselessResponsive' }],
      },
    ],
  })
  tester.run('no-duplicate-value composition', noDuplicateValue, {
    valid,
    invalid: [],
  })
  tester.run('no-useless-tailing-nulls composition', noUselessTailingNulls, {
    valid,
    invalid: [],
  })
})
