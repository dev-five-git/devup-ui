import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { styleOrderRange } from '../index'

const prefix = 'import {css,globalCss} from "@devup-ui/react";'
const splitToken = prefix + 'css`style-order:2/**/5;color:red`'
const invalidHole = prefix + 'css`style-order:/*${runtime}*/${false}/*tail*/`'

describe('metadata comment boundaries when interpolations are hidden', () => {
  const tester = new RuleTester()
  tester.run('literal comment visibility', styleOrderRange, {
    valid: [
      'css`style-order:2/*${runtime}*/;color:red`',
      'css`style-order:${2}/*${runtime}*/;color:red`',
      'css`style-order:/*${runtime}*/${2}/*${other}*/`',
      'css`style-order:/*head*/ +2.0 /*tail*/`',
      'globalCss`body{style-order:2/*${runtime}*/}`',
      'globalCss`body{style-order:/*${runtime}*/${2}}`',
      'css`content:"/*${runtime}*/";--data:{style-order:${runtime}};style-order:2`',
      'css`/*${runtime}*/style-order:2`',
      'css`style-order:${2}/*${runtime}`',
    ].map((tail) => ({ code: prefix + tail })),
    invalid: [
      {
        code: splitToken,
        errors: [
          {
            messageId: 'styleOrderRange',
            column: splitToken.indexOf('2/**/') + 1,
            endColumn: splitToken.indexOf(';color') + 1,
          },
        ],
      },
      {
        code: invalidHole,
        errors: [
          {
            messageId: 'styleOrderRange',
            column: invalidHole.indexOf('false') + 1,
            endColumn: invalidHole.indexOf('false') + 6,
          },
        ],
      },
      ...[
        'css`style-order:+/**/2`',
        'css`style-order:2./**/0`',
        'css`style-order:2e/**/0`',
        'css`style-order:2/*${runtime}*/5`',
        'css`style-order:2/*${runtime}*/${5}`',
        'css`style-order:${2}/*${runtime}*/5`',
        'css`style-order:"${2}/*${runtime}*/"`',
        'css`style-order:/*${runtime}*/${"02"}`',
        'css`style-order:/*${runtime}*/${null}`',
        'css`style-order:/*${runtime}*/${undefined}`',
      ].map((tail) => ({
        code: prefix + tail,
        errors: [{ messageId: 'styleOrderRange' }],
      })),
      {
        code: prefix + 'globalCss`body{style-order:${on?2:3}/*${runtime}*/}`',
        errors: [{ messageId: 'globalOrder' }],
      },
    ],
  })
})
