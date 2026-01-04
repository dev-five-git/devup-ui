import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { noUselessResponsive } from '../index'

describe('no-useless-responsive rule', () => {
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
  ruleTester.run('no-useless-responsive rule', noUselessResponsive, {
    valid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={1} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={"1"} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { "Box" as B } from "@devup-ui/react";\n<B w={[]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import B from "@devup-ui/react";\n<B.Box w={[]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import * as B from "@devup-ui/react";\n<B.Box w={[]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1][0]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, 3]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1, 2, 3][1]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box onClick={() => {console.log([1])}} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1].length === 1 ? 1 : 2} />',
        filename: 'src/app/page.tsx',
      },
      {
        // normal case
        code: '<Box w={[1]} />',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({w: 1})',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({w: "1"})',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "other-package";\ncss({w: [1][0]})',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { css } from "other-package";\ncss()',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { globalCss } from "@devup-ui/react";\nglobalCss({ imports: ["@devup-ui/react/css/global.css"] })',
        filename: 'src/app/page.tsx',
      },
      {
        code: 'import { globalCss } from "@devup-ui/react";\nglobalCss({ imports: [{"url": "@devup-ui/react/css/global.css"}] })',
        filename: 'src/app/page.tsx',
      },
    ],
    invalid: [
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={[1]} />',
        output: 'import { Box } from "@devup-ui/react";\n<Box w={1} />',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
      {
        code: 'import { Box } from "@devup-ui/react";\n<Box w={([1])} />',
        output: 'import { Box } from "@devup-ui/react";\n<Box w={(1)} />',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
      {
        code: 'import A from "@devup-ui/react";\n<A.Box w={([1])} />',
        output: 'import A from "@devup-ui/react";\n<A.Box w={(1)} />',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
      {
        code: 'import { css } from "@devup-ui/react";\ncss({w: [1]})',
        output: 'import { css } from "@devup-ui/react";\ncss({w: 1})',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
      {
        code: 'import { css as c } from "@devup-ui/react";\nc({w: [1]})',
        output: 'import { css as c } from "@devup-ui/react";\nc({w: 1})',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
      {
        code: 'import c from "@devup-ui/react";\nc.css({w: [1]})',
        output: 'import c from "@devup-ui/react";\nc.css({w: 1})',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
      {
        code: 'import * as c from "@devup-ui/react";\nc.css({w: [1]})',
        output: 'import * as c from "@devup-ui/react";\nc.css({w: 1})',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
      {
        code: 'import * as c from "@devup-ui/react";\nc.css({w: [1].length === 1 ? [1] : [2]})',
        output:
          'import * as c from "@devup-ui/react";\nc.css({w: [1].length === 1 ? 1 : 2})',
        filename: 'src/app/layout.tsx',
        errors: [
          {
            messageId: 'uselessResponsive',
          },
          {
            messageId: 'uselessResponsive',
          },
        ],
      },
    ],
  })
})
