import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { styleOrderRange } from '../index'

const filename = 'src/lexical.tsx'
const prefix = 'import {css,globalCss,styled} from "@devup-ui/react";'
const finiteBranch = prefix + 'css`style-order:${on?2:0};color:red`'
const accepted = [
  'style-order:"2"',
  "styleOrder:'254'",
  'style-order:"\\32 "',
  'style-order:"\\000032"',
  'style-order:"\\\n2"',
  'style-order:2/*tail*/ ',
  '/*head*/ style-order: /*value*/ 2 /*tail*/ ',
  'color:red; /* style-order:0 */ content:"style-order:0"',
  'content:a\\;style-order:0;style-order:2',
  'content:"escaped\\";style-order:0";style-order:2',
  '--data:{a:{style-order:0};b:2};style-order:2',
  'color:red/*unclosed',
  'content:"/*unclosed";style-order:2',
  'style-order:2;/*unclosed',
  'content:"/*style-order:0*/";style-order:2',
  'style-order:2e0',
  'style-order:.2e1',
  'style\\2d order:2',
]
const rejected = [
  'style-order:"02"',
  'style-order:"+2"',
  'style-order:"2.0"',
  'style-order:"\\0"',
  'style-order:"\\110000"',
  'style-order:"\\d800"',
  'style-order:"\\q"',
  'style-order:2px',
  'style-order:2!important',
  'style-order:2*/',
  'style-order:',
  'style-order:"/*2*/"',
  'content:url(data:image/svg+xml,{style-order:2});style-order:0',
  '[style-order="0"]{style-order:0}',
  'style\\2d order:0',
]

describe('literal lexical boundaries when punctuation is data', () => {
  const tester = new RuleTester({
    languageOptions: { parserOptions: { ecmaFeatures: { jsx: true } } },
  })
  tester.run(
    'quotes comments escapes and original positions',
    styleOrderRange,
    {
      valid: [
        ...accepted.map((text) => ({
          filename,
          code: `${prefix}css(${JSON.stringify(text)})`,
        })),
        { filename, code: prefix + 'css("style-order:\\x32")' },
        { filename, code: prefix + 'css("style-order:\\u0032")' },
        { filename, code: prefix + 'css("content:\\u{1f600};style-order:2")' },
        { filename, code: prefix + 'css("color:red;\\\r\nstyle-order:2")' },
        { filename, code: prefix + 'css("color:red;\\\nstyle-order:2")' },
        {
          filename,
          code: prefix + 'css`content:\\n\\r\\t\\b\\f\\v\\0\\q;style-order:2`',
        },
        { filename, code: prefix + 'css`color:red;\r\nstyle-order:2`' },
        {
          filename,
          code: prefix + 'styled.div`style-order:${()=>{const v=2;return v}}`',
        },
        { filename, code: prefix + 'css`${{color:"red"}}style-order:2`' },
        { filename, code: prefix + 'css`style-order:"${on?2:3}"`' },
        { filename, code: prefix + 'globalCss`body{style-order:"${2}"}`' },
      ],
      invalid: [
        {
          filename,
          code: finiteBranch,
          errors: [
            {
              messageId: 'styleOrderRange',
              column: finiteBranch.indexOf(':0') + 2,
              endColumn: finiteBranch.indexOf(':0') + 3,
            },
          ],
        },
        ...rejected.map((text) => ({
          filename,
          code: `${prefix}css(${JSON.stringify(text)})`,
          errors: [{ messageId: 'styleOrderRange' }],
        })),
        ...[
          'const order=order;css`style-order:1${order}`',
          'css`style-order:1${on&&255}`',
          'css`style-order:1${on?2:runtime}`',
          'css`style-order:1${on||2}`',
          'css`style-order:1${+runtime}`',
          'css`style-order:\\uZZZZ`',
          'css`style-order:"${2}/*data*/"`',
          'css`style-order:"${"02"}"`',
          'globalCss`body{style-order:1${on&&2}}`',
        ].map((tail) => ({
          filename,
          code: prefix + tail,
          errors: [
            {
              messageId:
                tail.startsWith('globalCss') && !tail.includes('on&&')
                  ? 'globalOrder'
                  : 'styleOrderRange',
            },
          ],
        })),
        {
          filename,
          code: 'import {css} from "@devup-ui/react";\r\ncss`content:"한😀";\r\n/* comment */style-order: /*value*/ 0;`',
          errors: [
            {
              messageId: 'styleOrderRange',
              line: 3,
              column: 37,
              endLine: 3,
              endColumn: 38,
            },
          ],
        },
        {
          filename,
          code: 'import {globalCss} from "@devup-ui/react";\nglobalCss`@font-face{\n/*head*/style-order:0}`',
          errors: [
            {
              messageId: 'unsupportedOrder',
              line: 3,
              column: 9,
              endLine: 3,
              endColumn: 20,
            },
          ],
        },
        {
          filename,
          code: 'import {css} from "@devup-ui/react";\ncss("content:\\u{1f600};style-order:0")',
          errors: [
            {
              messageId: 'styleOrderRange',
              line: 2,
              column: 36,
              endLine: 2,
              endColumn: 37,
            },
          ],
        },
      ],
    },
  )
})
