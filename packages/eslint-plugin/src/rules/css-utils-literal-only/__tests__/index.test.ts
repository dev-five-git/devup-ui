import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

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
        {
          code: `import { ${code} } from "@devup-ui/react";\nimport { SIZE, tokens } from "./tokens";\n${code}({w: SIZE, h: tokens.size})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\nconst S = 4;\nconst U = \`\${S * 2}px\`;\nconst N = -S;\nconst O = { a: 'red' } as const;\nconst T = 'x' satisfies string;\n${code}({w: S, h: U, m: N, color: O.a, content: T})`,
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
        ...[
          'let L = 1;',
          'const L = f();',
          'const L = L2; const L2 = L;',
          'const { L } = o;',
          'const L = !x;',
          'const L = { [k]: 1 };',
          'const L = { ...o };',
          'const L = o[k];',
          'function f(L) { return L }',
          'const L = 1 + g();',
        ].map((setup) => ({
          code: `import { ${code} } from "@devup-ui/react";\n${setup}\n${code}({w: L})`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
        })),
        {
          code: `import { ${code} } from "@devup-ui/react";\nfunction f() { const L = 1; return ${code}({w: L}) }`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' }],
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
