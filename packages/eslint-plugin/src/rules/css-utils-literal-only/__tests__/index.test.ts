import { RuleTester } from '@typescript-eslint/rule-tester'

import { cssUtilsLiteralOnly } from '../index'

describe.each(['css' /* 'globalCss', 'keyframes'*/])(
  'css-utils-literal-only rule',
  (code) => {
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
    ruleTester.run('css-utils-literal-only rule', cssUtilsLiteralOnly, {
      valid: [
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: 1})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: "1"})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} } from "other-package";\n${code}({w: [1][0]})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: [1]})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: ["1"]})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({w: ["1"]})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({_hover: {w: ["1"]}})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({ w: { a: 1, b: 2 }[v]})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({ w: v ? 1 : null})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({ w: v ? 1 : undefined})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({ w: v || 1 ? 1 : null})`,
          filename: 'src/app/page.tsx',
        },
      ],
      invalid: [
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: v})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: [v]})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: [1, null, v]})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({w: [1, null, v]})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({w: v ? 1 : v})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({w: v || 1 ? 1 : v})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
      ],
    })
  },
)
