import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { noRuntimeRead } from '../index'

describe('no-runtime-read rule', () => {
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const filename = 'src/app/page.tsx'
  const devup = `import { css, globalCss, keyframes, styled, Box, Text } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\n`
  const emotion = `import { css, keyframes, Global } from "@emotion/react";\nimport styled from "@emotion/styled";\n`
  const styledComponents = `import styled, { css as sc, createGlobalStyle } from "styled-components";\n`
  const read = (name: string) => ({
    messageId: 'noRuntimeRead' as const,
    data: { name },
  })

  ruleTester.run('no-runtime-read rule', noRuntimeRead, {
    valid: [
      ...[
        'css({ p: 1 })',
        'css`color: red;`',
        'css(base, { p: 1 })',
        'globalCss({ body: { p: 1 } })',
        'keyframes({ from: { p: 1 } })',
        'styled.div({ p: 1 })',
        'styled.div`color: red;`',
        'styled("div")({ p: 1 })',
        'styled("div")`color: red;`',
        'styled("div", { p: 1 })',
        'styled(Box)({ p: 1 })',
        'styled(Box)`color: red;`',
        'styled(Box, { p: 1 })',
        'styled.div.attrs({ a: 1 })({ p: 1 })',
        'styled.div.withConfig({}).attrs({})`color: red;`',
        '<Box p={1} />',
        '<Box p={1}>text</Box>',
        '<Box as={Text} />',
        '<Box as="a" />',
        '<Box as />',
        '<Text as={Box} />',
        'const alias = css;\nalias({ p: 1 })',
        'const Aliased = Box;\n<Aliased />',
        'const make = styled;\nmake.div({ p: 1 })',
        'const first = css, second = first;\nsecond({ p: 1 })',
        'const a = css;\nconst b = a;\nb({ p: 1 })',
        'Devup.css({ p: 1 })',
        'const namespace = Devup',
        'const member = Devup.css',
        'type Css = typeof css;\nlet value: typeof Box',
        'function f(css: string) { return css }',
        'function f() { const Box = 1; return Box }',
        'const other = { css: 1 };\nother.css',
        'foo({ css: 1 })',
      ].map((code) => ({ code: devup + code, filename })),
      {
        code: `${emotion}css({ p: 1 });\nstyled.div({});\n<Global styles={{}} />;\nkeyframes({})`,
        filename,
      },
      {
        code: `${styledComponents}sc({ p: 1 });\nstyled.div({});\ncreateGlobalStyle\`body {}\``,
        filename,
      },
      {
        code: `import { css } from "other-package";\nexport const x = css;\nconst y = [css]`,
        filename,
      },
      {
        code: `import { css, jsx } from "@emotion/react";\nexport const x = jsx;\ncss({})`,
        filename,
      },
      {
        code: `import { style } from "@vanilla-extract/css";\nexport const x = style({ p: 1 })`,
        filename,
      },
      {
        code: `import { css } from "@devup-ui/react";\nexport const x = css;`,
        filename: 'src/styles.css.ts',
      },
      { code: 'export const x = css', filename },
    ],
    invalid: [
      {
        code: `${devup}export const runtime = css`,
        filename,
        errors: [read('css')],
      },
      ...[
        ['export const a = globalCss', 'globalCss'],
        ['export const a = [keyframes]', 'keyframes'],
        ['export default css', 'css'],
        ['export { css }', 'css'],
        ['export { css as other }', 'css'],
        ['export const a = css.foo', 'css'],
        ['export const a = css[0]', 'css'],
        ['foo(css)', 'css'],
        ['export const a = { css }', 'css'],
        ['const alias = css;\nexport { alias }', 'alias'],
        ['export const a = cond ? css : null', 'css'],
        ['function f() {\n  const q = css;\n  return q({})\n}', 'css'],
        ['let q = css;\nq = 1', 'q'],
        ['export const alias = css', 'css'],
        ['const a = css;\nfoo(a)', 'a'],
        ['String(css)', 'css'],
        ['export const a = styled', 'styled'],
        ['export const a = styled.div', 'styled'],
        ['export const a = styled("div")', 'styled'],
        ['export const a = styled.div.attrs({})', 'styled'],
        ['export const a = styled.div.withConfig({})', 'styled'],
        ['export const a = styled["div"]({ p: 1 })', 'styled'],
        ['export const a = styled.div.foo', 'styled'],
        ['const base = styled.div;\nexport { base }', 'styled'],
        ['export const a = styled`color: red;`', 'styled'],
        ['export const a = styled(Box)', 'styled', 'Box'],
        ['export const a = <div x={Box} />', 'Box'],
        ['export const a = <div as={Box} />', 'Box'],
        ['export const a = <Box as={css} />', 'css'],
        ['export const a = <Box.Foo />', 'Box'],
        ['export const a = [Box].length', 'Box'],
        ['export const a = React.createElement(Box, {})', 'Box'],
        ['export const a = foo(Box)', 'Box'],
        ['export const a = styled(foo, Box)', 'Box'],
        ['const Aliased = Box;\nfoo(Aliased)', 'Aliased'],
      ].map(([code, ...names]) => ({
        code: devup + code,
        filename,
        errors: names.map(read),
      })),
      {
        code: `${emotion}export const a = [css, styled];\nexport const b = Global`,
        filename,
        errors: [read('css'), read('styled')],
      },
      {
        code: `${styledComponents}export const a = sc;\nexport const b = styled.div;\nexport const c = createGlobalStyle`,
        filename,
        errors: [read('sc'), read('styled'), read('createGlobalStyle')],
      },
      {
        code: `import { style, globalStyle } from "@vanilla-extract/css";\nexport const a = [style, globalStyle]`,
        filename,
        errors: [read('style'), read('globalStyle')],
      },
      {
        code: `import styled, { css } from "styled-components";\nimport * as Emotion from "@emotion/styled";\nexport const a = [Emotion, css]`,
        filename,
        errors: [read('Emotion'), read('css')],
      },
    ],
  })
})
