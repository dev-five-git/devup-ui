import { RuleTester } from '@typescript-eslint/rule-tester'

import { styleOrderRange } from '../index'

describe('style-order-range rule', () => {
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
  })

  ruleTester.run('style-order-range rule', styleOrderRange, {
    valid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={1} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={254} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={128} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="1" />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="254" />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="100" />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={50} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={`50`} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="200" />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={+200} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box otherProp={300} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: 1})',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'css({styleOrder: 1})',
        filename: 'src/app/page.tsx',
      },
      {
        code: '<Box styleOrder={300} />',
        filename: 'src/app/page.tsx',
      },
    ],
    invalid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={0} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={255} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={-1} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="-5" />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="0" />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="255" />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={256} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="300" />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="abc" />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={undefined} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={null} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={-100} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder="1000" />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={`1000`} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder={someVariable} />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: someVariable})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: `someVariable`})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: 1000})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: -100})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: "1000"})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: `1000`})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: `${someVariable}`})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: +someVariable})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: -someVariable})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: typeof `100`})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: void `100`})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: delete `100`})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: ~ `100`})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: `100` + 100})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({styleOrder: ~10})',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'styleOrderRange',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box styleOrder=<div /> />',
        filename: 'src/app/page.tsx',
        errors: [
          {
            messageId: 'wrongType',
          },
        ],
      },
    ],
  })
})
