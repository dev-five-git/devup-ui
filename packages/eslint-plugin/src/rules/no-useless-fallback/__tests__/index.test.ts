import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { noUselessFallback } from '../index'

describe('no-useless-fallback rule', () => {
  const tester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const emotion = "import { css } from '@emotion/react'\n"
  const valid = [
    // A newer value stands in for an older one a browser may not support
    "{ height: ['100vh', '100dvh'] }",
    "{ height: ['100vh', '100svh'] }",
    "{ width: ['100%', '50cqw'] }",
    "{ marginTop: ['1.5em', '1lh'] }",
    "{ color: ['red', 'oklch(0.6 0.2 250)'] }",
    "{ color: ['red', 'OKLAB(0.6 0.1 0.1)'] }",
    "{ color: ['#f00', 'lab(50% 40 59)'] }",
    "{ color: ['#f00', 'lch(50% 40 59)'] }",
    "{ color: ['#f00', 'hwb(0 0% 0%)'] }",
    "{ color: ['#f00', 'color(display-p3 1 0 0)'] }",
    "{ color: ['red', 'color-mix(in srgb, red, blue)'] }",
    "{ color: ['black', 'light-dark(black, white)'] }",
    "{ color: ['#f00', 'rgb(255 0 0 / 50%)'] }",
    "{ color: ['#f00', 'hsl(0deg 100% 50%)'] }",
    "{ fontSize: ['16px', 'clamp(1rem, 2vw, 2rem)'] }",
    "{ width: ['100%', 'min(100%, 40rem)'] }",
    "{ width: ['100%', 'max(50%, 20rem)'] }",
    "{ color: ['red', 'var(--brand)'] }",
    "{ paddingTop: ['0', 'env(safe-area-inset-top)'] }",
    "{ display: ['-webkit-box', 'flex'] }",
    "{ position: ['-webkit-sticky', 'sticky'] }",
    "{ width: ['-moz-fit-content', 'fit-content'] }",
    "{ cursor: ['pointer', '-o-grab'] }",
    "{ display: ['-ms-flexbox', 'flex'] }",
    // Not every value is a literal string
    "{ margin: ['1px', 3] }",
    "{ margin: ['1px', size] }",
    "{ margin: ['1px', `${size}px`] }",
    "{ color: ['red', null, 'blue'] }",
    "{ color: ['red', , 'blue'] }",
    "{ color: ['red', ...rest] }",
    "{ color: ['', 'blue'] }",
    // One value, or none
    "{ color: ['red'] }",
    '{ color: [] }',
    // Devup UI shorthands and keys holding no CSS value read arrays as breakpoints
    "{ p: ['1px', '2px'] }",
    "{ bg: ['red', 'blue'] }",
    "{ typography: ['h1', 'h2'] }",
    "{ _hover: ['red', 'blue'] }",
    "{ '&:hover': ['red', 'blue'] }",
    "{ ':focus': ['red', 'blue'] }",
    "{ '@media (min-width: 1px)': ['red', 'blue'] }",
    "{ [key]: ['red', 'blue'] }",
    "{ color: { nested: ['red', 'blue'] }.nested }",
  ].map((rules) => ({ code: `${emotion}css(${rules})`, filename: 'page.tsx' }))
  const elsewhere = [
    // styled-components joins the values with commas into one declaration
    "import styled, { css } from 'styled-components'\ncss({ fontFamily: ['Arial', 'sans-serif'] })\nstyled.div({ margin: ['1px', '3px'] })",
    // Devup UI reads arrays as breakpoints
    "import { css, Box } from '@devup-ui/react'\ncss({ color: ['red', 'blue'] })\n;<Box color={['red', 'blue']} />",
    // Packages added with importAliases read arrays as Devup UI does
    {
      code: "import { css } from 'acme-css'\ncss({ color: ['red', 'blue'] })",
      settings: { '@devup-ui': { importAliases: { 'acme-css': true } } },
    },
    // Not a style
    "import { css } from '@emotion/react'\nconst data = { color: ['red', 'blue'] }\nexport { data }",
    "import { css } from '@emotion/react'\ncss([{ color: 'red' }, 'blue'].map(String))",
    "const rules = { color: ['red', 'blue'] }",
  ].map((test) =>
    typeof test === 'string'
      ? { code: test, filename: 'page.tsx' }
      : { ...test, filename: 'page.tsx' },
  )

  tester.run('no-useless-fallback', noUselessFallback, {
    valid: [...valid, ...elsewhere],
    invalid: [
      {
        code: `${emotion}css({ margin: ['1px', '3px'] })`,
        filename: 'page.tsx',
        errors: [
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/react', last: '"3px"' },
          },
        ],
      },
      {
        code: `${emotion}css({ color: ['red', \`blue\`], '--gap': ['1px', '2px'], backgroundColor: ['#fff', 'rgb(0, 0, 0)', 'rgba(0,0,0,.5)'] })`,
        filename: 'page.tsx',
        errors: [
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/react', last: '"blue"' },
          },
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/react', last: '"2px"' },
          },
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/react', last: '"rgba(0,0,0,.5)"' },
          },
        ],
      },
      {
        code: `${emotion}css({ height: ['100vh', 'calc(100vh - 1px)'], fontSize: ['16px', '1rem'], display: ['block', 'grid'] })`,
        filename: 'page.tsx',
        errors: [
          { messageId: 'uselessFallback' },
          { messageId: 'uselessFallback' },
          { messageId: 'uselessFallback' },
        ],
      },
      {
        code: "import styled from '@emotion/styled'\nimport { Global, keyframes } from '@emotion/react'\nconst base = { margin: ['1px', '2px'] }\nstyled.div(base)\nstyled('span')({ ':hover': { color: ['red', 'blue'] } })\nkeyframes({ from: { opacity: ['0', '1'] } })\n;<Global styles={{ body: { margin: ['0', '8px'] } }} />",
        filename: 'page.tsx',
        errors: [
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/styled', last: '"2px"' },
          },
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/styled', last: '"blue"' },
          },
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/react', last: '"1"' },
          },
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/react', last: '"8px"' },
          },
        ],
      },
      {
        code: "import { css, injectGlobal } from '@emotion/css'\ncss({ color: ['red', 'blue'] })\ninjectGlobal({ body: { color: ['red', 'blue'] } })",
        filename: 'page.tsx',
        errors: [
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/css', last: '"blue"' },
          },
          {
            messageId: 'uselessFallback',
            data: { library: '@emotion/css', last: '"blue"' },
          },
        ],
      },
      {
        code: "import { style, globalStyle } from '@vanilla-extract/css'\nstyle({ margin: ['1px', '3px'] })\nglobalStyle('body', { color: ['red', 'blue'] })",
        filename: 'page.tsx',
        errors: [
          {
            messageId: 'uselessFallback',
            data: { library: '@vanilla-extract/css', last: '"3px"' },
          },
          {
            messageId: 'uselessFallback',
            data: { library: '@vanilla-extract/css', last: '"blue"' },
          },
        ],
      },
    ],
  })
})
