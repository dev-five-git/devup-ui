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
        {
          code: `import { ${code} } from "@devup-ui/react";\nimport { darken, PRIMARY } from "./color";\nimport * as tokens from "./tokens";\nfunction tone(n) { return n * 2 }\nconst toneOf = (n) => n;\nconst S = 4;\nconst DARK = darken(0.1, PRIMARY);\nconst HALF = Math.round(S / 2);\nconst PICK = [1, 2][1];\nconst SIDE = S > 2 ? 'left' : 'right';\nconst EITHER = S || 1;\nconst TEXT = String(S);\nconst LIMIT = -Infinity;\n${code}({color: DARK, w: HALF, h: PICK, float: SIDE, m: EITHER, content: TEXT, zIndex: LIMIT, p: Math.max(S, 2), top: Math.PI, left: tone(S), right: toneOf(S), bottom: tokens.scale(S), order: undefined})`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\n${code}({w: [1, 2].map((n) => n * 4)[1]})`,
          filename: 'src/app/page.tsx',
        },
        ...[
          'L.p.toString();',
          'L.list.map(String);',
          'f(L.p);',
          'for (const k in L) {}',
          'for (const v of L.list) {}',
          'L[k]();',
          'Object.keys(L);',
          'Object.assign({}, L);',
          'o.assign(L, {});',
          'Object[k](L);',
          'x = L.p;',
          'delete o.p;',
          'let [a = L.p] = [];',
        ].map((use) => ({
          code: `import { ${code} } from "@devup-ui/react";\nconst L = { p: 1, list: [1] };\n${use}\n${code}({w: L.p})`,
          filename: 'src/app/page.tsx',
        })),
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
          'const L = Date.now();',
          'const L = Math.random();',
          'const L = Math.random;',
          'const L = [...o];',
          'const L = [1, , 2];',
          'const L = (() => 1)();',
          'function t(n) { return n }\nconst L = t(...[1]);',
          'const Math = { max: () => o };\nconst L = Math.max(1);',
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
        ...[
          'w: Math.random()',
          'w: Date.now()',
          'w: Math.random',
          'w: o.f(1)',
          'w: fn(1)',
        ].map((value) => ({
          code: `import { ${code} } from "@devup-ui/react";\nlet fn = () => 1;\n${code}({${value}})`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
        })),
        {
          code: `import { ${code} } from "@devup-ui/react";\nfunction f(g) { return ${code}({w: g(1)}) }`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' }],
        },
        {
          code: `import { ${code} } from "@devup-ui/react";\nfunction f(k) { return ${code}({w: [1, 2].map((n) => n * k)[1]}) }`,
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
        ...[
          'L.p = 2;',
          'L.p += 2;',
          'L.p++;',
          'delete L.p;',
          '[L.p] = [2];',
          '[...L.list] = [2];',
          '[L.p = 1] = [];',
          '({ a: L.p } = {});',
          'for (L.p in o) {}',
          'for (L.p of []) {}',
          'L.list.push(2);',
          'L.list.sort();',
          'Object.assign(L, {});',
          'Object.defineProperty(L, "p", {});',
          '(L as any).p = 2;',
          'L!.p = 2;',
          'L?.list.reverse();',
        ].map((change) => ({
          code: `import { ${code} } from "@devup-ui/react";\nconst L = { p: 1, list: [1] };\n${change}\n${code}({w: L.p})`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
        })),
        {
          code: `import { ${code} } from "@devup-ui/react";\nimport { base } from "./tokens";\nbase.p = 2;\n${code}({w: base.p})`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' }],
        },
      ],
    })
  },
)
