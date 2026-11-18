import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { noDuplicateValue } from '../../rules/no-duplicate-value'
import { noUselessResponsive } from '../../rules/no-useless-responsive'
import { noUselessTailingNulls } from '../../rules/no-useless-tailing-nulls'

/**
 * Emotion, styled-components and vanilla-extract rules declare a property's
 * array of literals as fallbacks in one rule (`color:red;color:blue`), so the
 * responsive rules leave them alone: `[4, 4, 8]` fixed to `[4, null, 8]`
 * would become breakpoints.
 */
describe('library rules take arrays as fallbacks', () => {
  const tester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const arrays = '{ color: ["red", "red", "blue"], m: [4], p: [1, null] }'
  const valid = [
    `import { css, keyframes, Global } from '@emotion/react'
css(${arrays})
keyframes({ from: ${arrays} })
const s = ${arrays}
css(s)
;<Global styles={{ body: ${arrays} }} />`,
    `import styled from '@emotion/styled'
import * as Namespaced from '@emotion/styled'
styled.div(${arrays})
styled('div')(${arrays})
styled(Base)(${arrays})
styled.div.attrs({})(${arrays})
Namespaced.div(${arrays})`,
    `import { css, keyframes, injectGlobal } from '@emotion/css'
css(${arrays})
keyframes({ from: ${arrays} })
injectGlobal({ body: ${arrays} })`,
    `import styled, { css, keyframes } from 'styled-components'
import * as S from 'styled-components'
css(${arrays})
keyframes({ from: ${arrays} })
styled.div(${arrays})
styled('div', ${arrays})
S.div(${arrays})`,
    `import { style, globalStyle, keyframes } from '@vanilla-extract/css'
style(${arrays})
globalStyle('body', ${arrays})
keyframes({ from: ${arrays} })`,
  ].map((code) => ({ code, filename: 'page.tsx' }))
  const devup = `import { css } from '@emotion/react'
import { css as devupCss, Box } from '@devup-ui/react'
css({ m: [4, 4, null] })
devupCss({ m: [4, 4, null] })
;<Box m={[4, 4, null]} />`
  const custom = {
    code: 'import { css } from "acme-css"\ncss({ m: [4, 4, null] })',
    filename: 'page.tsx',
    settings: { '@devup-ui': { importAliases: { 'acme-css': true } } },
  }

  tester.run('no-duplicate-value', noDuplicateValue, {
    valid,
    invalid: [
      {
        code: devup,
        output: `import { css } from '@emotion/react'
import { css as devupCss, Box } from '@devup-ui/react'
css({ m: [4, 4, null] })
devupCss({ m: [4, null, null] })
;<Box m={[4, null, null]} />`,
        filename: 'page.tsx',
        errors: [
          { messageId: 'duplicateValue' },
          { messageId: 'duplicateValue' },
        ],
      },
      {
        ...custom,
        output: 'import { css } from "acme-css"\ncss({ m: [4, null, null] })',
        errors: [{ messageId: 'duplicateValue' }],
      },
    ],
  })
  tester.run('no-useless-tailing-nulls', noUselessTailingNulls, {
    valid,
    invalid: [
      {
        ...custom,
        output: 'import { css } from "acme-css"\ncss({ m: [4, 4] })',
        errors: [{ messageId: 'uselessTailingNulls' }],
      },
    ],
  })
  tester.run('no-useless-responsive', noUselessResponsive, {
    valid,
    invalid: [
      {
        code: 'import { css } from "@emotion/react"\nimport { Box } from "@devup-ui/react"\ncss({ m: [4] })\n;<Box m={[4]} />',
        output:
          'import { css } from "@emotion/react"\nimport { Box } from "@devup-ui/react"\ncss({ m: [4] })\n;<Box m={4} />',
        filename: 'page.tsx',
        errors: [{ messageId: 'uselessResponsive' }],
      },
    ],
  })
})
