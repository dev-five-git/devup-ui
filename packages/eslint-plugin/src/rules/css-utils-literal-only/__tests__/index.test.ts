import { RuleTester } from '@typescript-eslint/rule-tester'

import { cssUtilsLiteralOnly } from '../index'

describe('css-utils-literal-only rule', () => {
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
  it.each(['css', 'globalCss'])('should pass', (code) => {
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
      ],
      invalid: [
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: v})`,
          output: `import { ${code} } from "@devup-ui/react";\n${code}({w: v})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: [v]})`,
          output: `import { ${code} } from "@devup-ui/react";\n${code}({w: [v]})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: [1, null, v]})`,
          output: `import { ${code} } from "@devup-ui/react";\n${code}({w: [1, null, v]})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
        {
          code: `import { ${code} as B } from "@devup-ui/react";\nB({w: [1, null, v]})`,
          output: `import { ${code} as B } from "@devup-ui/react";\nB({w: [1, null, v]})`,
          filename: 'src/app/layout.tsx',
          errors: [
            {
              messageId: 'cssUtilsLiteralOnly',
            },
          ],
        },
      ],
    })
  })
})
