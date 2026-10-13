import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import cases from '../../../../../../test-fixtures/style-order-w38.json'
import { styleOrderRange } from '../index'

const filename = 'src/literal.tsx'
const prefix =
  'import { css, styled, globalCss, keyframes } from "@devup-ui/react";'

describe('literal metadata when consumed as CSS text', () => {
  const tester = new RuleTester({
    languageOptions: { parserOptions: { ecmaFeatures: { jsx: true } } },
  })
  tester.run(
    'literal directives preserve primitive and scope semantics',
    styleOrderRange,
    {
      valid: [
        ...cases
          .filter((entry) => entry.valid)
          .map(
            (entry) =>
              `${prefix}css\`style-order:\${${entry.expression}};color:red\``,
          ),
        `${prefix}css\`style-order: +2.0; color:red\``,
        `${prefix}css\`style-order:\${on ? 2 : 3};color:red\``,
        `${prefix}css\`style-order:1\${on ? 2 : 3};color:red\``,
        `${prefix}globalCss\`body{style-order:2;color:red}\``,
        `${prefix}css\`content:";style-order:0;{}";--style-order:0;--data:{style-order:0};background:url("style-order:0");[style-order="0"]{color:red} styleOrder{color:red}\``,
        `${prefix}css({content:"style-order:0",props:{text:"style-order:0"}})`,
        `${prefix}keyframes({from:{content:"style-order:0"}})`,
        `${prefix}globalCss({fontFaces:[{fontFamily:"style-order:0"}]})`,
        `${prefix}function f(css,styled){css\`style-order:0\`;styled.div\`style-order:0\`}`,
      ].map((code) => ({ filename, code })),
      invalid: [
        ...cases
          .filter((entry) => !entry.valid)
          .map((entry) => ({
            filename,
            code: `${prefix}css\`style-order:\${${entry.expression}};color:red\``,
            errors: [{ messageId: 'styleOrderRange' }],
          })),
        ...[
          'css`style-order:0;color:red`',
          'styled.div`style-order:0;color:red`',
          'css("style-order:0;color:red")',
          'css({selectors:{"&:hover":"style-order:0;color:red"}})',
        ].map((tail) => ({
          filename,
          code: prefix + tail,
          errors: [{ messageId: 'styleOrderRange' }],
        })),
        {
          filename,
          code: prefix + 'keyframes`from{style-order:2;opacity:0}`',
          errors: [{ messageId: 'unsupportedOrder' }],
        },
        {
          filename,
          code: prefix + 'globalCss`body{style-order:${on?2:3};color:red}`',
          errors: [{ messageId: 'globalOrder' }],
        },
      ],
    },
  )
})
