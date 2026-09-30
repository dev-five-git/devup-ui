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
          code: `import { ${code} } from "@devup-ui/react";\nimport { PRIMARY, SIZE } from "./color";\nfunction tone(n: number): number { return (n as number) * 2 }\nconst toneOf = (n) => n;\nconst S = 4;\nconst DARK = \`\${PRIMARY}\`;\nconst HALF = Math.round(S / 2);\nconst PICK = [1, 2][1];\nconst SIDE = S > 2 ? 'left' : 'right';\nconst EITHER = S || 1;\nconst TEXT = String(S);\nconst LIMIT = -Infinity;\nconst scale = (n) => { const o = { a: n }; o.a++; delete o.b; return tone(o.a) * SIZE; };\n${code}({color: DARK, w: HALF, h: PICK, float: SIDE, m: EITHER, content: TEXT, zIndex: LIMIT, p: Math.max(S, 2), top: Math.PI, left: tone(S), right: toneOf(S), bottom: scale(S), order: undefined, opacity: 'a'.toUpperCase().length})`,
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
          'L.list.forEach((n) => n);',
          'L.arrow();',
          'L.plain();',
          'Object.keys(L);',
          'Object.freeze(L);',
          'String(L);',
          'JSON.stringify(L);',
          'x = L.p;',
          'delete o.p;',
          'let [a = L.p] = [];',
          'const M = L;',
          'export const N = { inner: L };',
          'export default L;',
          'module.exports = L;',
          `${code}(L);`,
          '<Box {...L} />;',
          'o[L.p];',
        ].map((use) => ({
          code: `import { ${code}, Box } from "@devup-ui/react";\nconst L = { p: 1, list: [1], arrow: () => 1, plain() { return 1 } };\n${use}\n${code}({w: L.p})`,
          filename: 'src/app/page.tsx',
        })),
        // Parts `css()` composes as classes, and what chooses between parts
        ...[
          `${code}(cls)`,
          `${code}(base, { m: 1 })`,
          `${code}(on ? { p: 1 } : null)`,
          `${code}(on && { p: 1 })`,
          `${code}(on || { p: 1 })`,
          `${code}([cls, { p: 1 }])`,
          `${code}(...parts)`,
          `${code}(cls as string)`,
          `${code}(\`color: red;\`)`,
          `${code}\`color: \${'red'};\``,
          `Devup.${code}(cls, { p: 1 })`,
          `styled.div({ color: on })`,
          `styled('div', { color: on })`,
          `styled.div\`color: \${on};\``,
          `f({ color: on })`,
          `obj[k]({ color: on })`,
          `Devup.Box({ color: on })`,
          `${code}({ w: (255).toString().length, h: Math['max'](1, 2) })`,
          `const f = () => (255).toString().length + Math.max(1, 2);\n${code}({ w: f() })`,
          `const styles = { body: { m: 0 } };\nglobalCss(styles)`,
        ].map((use) => ({
          code: `import { ${code}, globalCss, styled } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\nlet cls = 'a', base = {}, on = true, parts = [];\n${use}`,
          filename: 'src/app/page.tsx',
        })),
        {
          code: `import { css, keyframes as kf, globalCss } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\nconst fade = kf({ from: { opacity: 0 } });\nconst spin = Devup.keyframes\`from { rotate: 0deg; }\`;\nconst base = css({ color: 'red' });\nconst tagged = css\`color: blue;\`;\ncss({ animationName: fade, animation: \`\${spin} 1s\`, selectors: { [\`.\${base} &\`]: { m: 1 } } });\nglobalCss({ body: { animationName: fade } });\nkf({ from: { opacity: 0 }, to: { content: \`"\${tagged}"\` } });`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import { css, keyframes } from "@devup-ui/react";\nexport function C() { const fade = keyframes({ from: { opacity: 0 } }); const spin = keyframes\`from { rotate: 0deg; }\`; return css({ animationName: fade, animation: \`\${spin} 1s\` }); }`,
          filename: 'src/app/page.tsx',
        },
        {
          code: `import * as stylex from "@stylexjs/stylex";\nimport sx, { create, defineVars as vars, props } from "@stylexjs/stylex";\nconst colors = stylex.defineVars({ c: 'red' });\nconst named = vars({ c: 'blue' });\nconst fade = stylex.keyframes({ from: { opacity: 0 } });\nconst styles = stylex.create({ a: { color: colors.c, animationName: fade, width: stylex.firstThatWorks('1px', 'auto') }, b: (w) => ({ width: w }) });\ncreate({ a: { color: named.c } });\nsx.create({ a: { color: 'red' } });\nstylex.createTheme(colors, { c: 'green' });\nstylex.props(styles.a, on);\nprops(on);`,
          filename: 'src/app/page.tsx',
        },
      ],
      invalid: [
        ...[
          [
            `export function C() { let fade = keyframes({ from: { opacity: 0 } }); return css({ animationName: fade }); }`,
            1,
          ],
          [
            `export function C(v) { const fade = keyframes({ from: { opacity: v } }); return css({ animationName: fade }); }`,
            2,
          ],
          [
            `export function C() { const fade = other({ from: { opacity: 0 } }); return css({ animationName: fade }); }`,
            1,
          ],
        ].map(([code, count]) => ({
          code: `import { css, keyframes } from "@devup-ui/react";\n${code}`,
          filename: 'src/app/page.tsx',
          errors: Array.from({ length: Number(count) }, () => ({
            messageId: 'cssUtilsLiteralOnly' as const,
          })),
        })),
        ...[
          [`let fade = keyframes({ from: { opacity: 0 } });`, 1],
          [`const fade = keyframes({ from: { opacity: v } });`, 2],
          [`const fade = keyframes\`from { opacity: \${v}; }\`;`, 2],
          [`const fade = other({ from: { opacity: 0 } });`, 1],
          [`const fade = Other.keyframes({ from: { opacity: 0 } });`, 1],
          [`const fade = other\`from { opacity: 0; }\`;`, 1],
        ].map(([declaration, count]) => ({
          code: `import { css, keyframes } from "@devup-ui/react";\nimport * as Other from "other";\nlet v = 1;\n${declaration}\ncss({ animationName: fade });`,
          filename: 'src/app/page.tsx',
          errors: Array.from({ length: Number(count) }, () => ({
            messageId: 'cssUtilsLiteralOnly' as const,
          })),
        })),
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
        ...[
          'L[k]();',
          'L.plain();',
          'Object.assign({}, L);',
          'o.assign(L, {});',
          'Object[k](L);',
          'f(L);',
          'f(L.items);',
          'for (const v of L.items) {}',
          'L.items.forEach((item) => { item.p = 2 });',
          'const M = L;\nM.p = 2;',
          'export const N = { inner: L };\nN.inner.p = 2;',
          'const M = [...L.items];',
          '<Other value={L} ref={L} />;',
          'tag`${L}`;',
          'new Thing(L);',
          'const g = () => L;',
          'let copy; copy = L;',
        ].map((use) => ({
          code: `import { ${code} } from "@devup-ui/react";\nconst L = { p: 1, items: [{ p: 1 }], plain() { this.p = 2 } };\n${use}\n${code}({w: L.p})`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
        })),
        ...[
          'import { darken } from "./color";\nconst L = darken(1);',
          'import * as tokens from "./tokens";\nconst L = tokens.scale(1);',
          'import { make } from "./tokens";\nconst f = (n) => make(n);\nconst L = f(1);',
          'import { box } from "./tokens";\nconst f = () => box.get();\nconst L = f();',
          'const L = Math.sin(1);',
          'const L = 2 ** 2;',
          'const L = (255).toString(16);',
          "const L = 'a'.normalize();",
          "const L = 'a'.toLocaleUpperCase();",
          'const L = /a/;',
          'const f = (s) => s.normalize();\nconst L = f("a");',
          'const f = ({ normalize }) => normalize;\nconst L = f("a");',
          'const f = () => typeof window;\nconst L = f();',
          'const f = () => { try { return 1 } catch { return 2 } };\nconst L = f();',
          'const f = async () => 1;\nconst L = f();',
          'function* g() { yield 1 }\nconst L = g();',
          'const f = () => new Map();\nconst L = f();',
          'const f = () => class {};\nconst L = f();',
          'const f = () => this;\nconst L = f();',
          'const f = () => import.meta.url;\nconst L = f();',
          'const f = () => import("./x");\nconst L = f();',
          'const f = () => <div />;\nconst L = f();',
          'const f = () => <></>;\nconst L = f();',
          'const f = () => /a/.source;\nconst L = f();',
          'const f = () => 2 ** 2;\nconst L = f();',
          'const f = () => { let n = 2; n **= 2; return n };\nconst L = f();',
          'const f = () => Math.sin(1);\nconst L = f();',
          'const f = (o) => o[k]();\nconst L = f(1);',
          'const f = () => (255).toString(16);\nconst L = f();',
          'const f = () => ({ get p() { return 1 } });\nconst L = f().p;',
          'const box = { n: 1 };\nconst f = () => { box.n = 2; return 1 };\nconst L = f();',
          'const box = { n: 1 };\nconst f = () => { box.n++; return 1 };\nconst L = f();',
          'const box = { n: 1 };\nconst f = () => { delete box.n; return 1 };\nconst L = f();',
          'let n = 1;\nconst f = () => n;\nconst L = f();',
          'const f = () => missing;\nconst L = f();',
          'const f = () => h();\nlet h = () => 1;\nconst L = f();',
          'const f = () => ({ n: 1 });\nf.n = 2;\nconst L = f();',
          'const f = () => 1;\nconst g = f;\nconst L = g();',
          'const { sin } = Math;\nconst L = sin(1);',
          'const f = () => { const { pow } = Math; return pow(2, 0.5) };\nconst L = f();',
          'const f = () => { const M = Math; return M.max(1, 2) };\nconst L = f();',
          'const f = () => (Math as any).max(1, 2);\nconst L = f();',
          "const k = 'max';\nconst L = Math[k];",
          'const t = (255).toString;\nconst L = t.call(255, 16);',
          'const f = () => { const t = (255).toString; return t.call(255, 16) };\nconst L = f();',
          'const f = () => { const { toString } = 255; return toString.call(255, 16) };\nconst L = f();',
          "const L = 'a'['toString'];",
        ].map((setup) => ({
          code: `import { ${code} } from "@devup-ui/react";\n${setup}\n${code}({w: L})`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
        })),
        // Values in every argument and in CSS text
        ...[
          `${code}(base, { color: v })`,
          `${code}({ m: 1 }, { color: v })`,
          `${code}(on ? { p: v } : null)`,
          `${code}(on && { p: v })`,
          `${code}([cls, { p: v }])`,
          `${code}(\`color: \${v};\`)`,
          `${code}\`color: \${v};\``,
          `Devup.${code}({ color: v })`,
          `globalCss({ body: { color: v } })`,
          `globalCss\`body { color: \${v}; }\``,
          `globalCss(v)`,
          `keyframes({ from: { opacity: v } })`,
          `keyframes\`from { opacity: \${v}; }\``,
          `createGlobalStyle(v)`,
        ].map((use) => ({
          code: `import { ${code}, globalCss, keyframes, createGlobalStyle } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\nlet cls = 'a', base = {}, on = true, v = 1;\n${use}`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
        })),
        ...[
          'stylex.create({ a: { color: v } })',
          'create({ a: { color: v } })',
          'stylex.keyframes({ from: { opacity: v } })',
          'stylex.createTheme(v, { c: "red" })',
          'sx.defineConsts({ c: v })',
        ].map((use) => ({
          code: `import * as stylex from "@stylexjs/stylex";\nimport sx, { create } from "@stylexjs/stylex";\nlet v = 1;\n${use}`,
          filename: 'src/app/layout.tsx',
          errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
        })),
      ],
    })
  },
)
