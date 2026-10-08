import { RuleTester } from '@typescript-eslint/rule-tester'

import { cssUtilsLiteralOnly } from '../index'

const forms = [
  ['import sx from "@stylexjs/stylex";', 'sx'],
  ['import { default as sx } from "@stylexjs/stylex";', 'sx'],
  ['import * as sx from "@stylexjs/stylex";', 'sx'],
  ['import * as upstream from "@stylexjs/stylex";', 'upstream.default'],
  ['import * as sx from "@devup-ui/react/stylex";', 'sx'],
  ['import { stylex as sx } from "@devup-ui/react";', 'sx'],
  ['import * as devup from "@devup-ui/react";', 'devup.stylex'],
  ['const sx = require("@stylexjs/stylex");', 'sx'],
  ['const { default: sx } = require("@stylexjs/stylex");', 'sx'],
  ['const sx = require("@devup-ui/react/stylex");', 'sx'],
  ['const devup = require("@devup-ui/react");', 'devup.stylex'],
  [
    'const { stylex: sx, getTheme = fallback } = require("@devup-ui/react");',
    'sx',
  ],
  [
    'import * as devup from "@devup-ui/react"; const sx = devup.stylex; const alias = sx;',
    'alias',
  ],
] as const
const apis = [
  'create',
  'keyframes',
  'defineVars',
  'defineConsts',
  'createTheme',
  'createThemeContract',
  'positionTry',
  'viewTransitionClass',
] as const
const calls = [
  ...forms.flatMap(([setup, namespace]) =>
    apis.map((api) => [setup, `${namespace}.${api}`]),
  ),
  ...['@stylexjs/stylex', '@devup-ui/react/stylex'].flatMap((source) =>
    apis.flatMap((api) => [
      [`import { ${api} as build } from "${source}";`, 'build'],
      [`const { ${api}: build } = require("${source}");`, 'build'],
      [`const { "${api}": build } = require("${source}");`, 'build'],
      [`import { "${api}" as build } from "${source}";`, 'build'],
      [
        `import * as sx from "${source}"; const build = sx.${api}; const alias = build;`,
        'alias',
      ],
    ]),
  ),
  [
    'import { stylex } from "@devup-ui/react"; const build = stylex.positionTry;',
    'build',
  ],
] as const
const ruleTester = new RuleTester()
ruleTester.run('StyleX binding parity', cssUtilsLiteralOnly, {
  valid: [
    ...calls.flatMap(([setup, callee]) => [
      { code: `${setup} ${callee}({ a: { color: "red" } });` },
      {
        code: `${setup} const value = "red"; ${callee}({ a: { color: value } });`,
      },
    ]),
    ...forms.map(([setup, namespace]) => ({
      code: `${setup}
        const tokens = ${namespace}.defineVars({ c: ${namespace}.types.color("red") });
        const frames = ${namespace}.keyframes({ from: { opacity: 0 } });
        const base = ${namespace}.create({ a: { color: tokens.c } });
        ${namespace}.create({ a: { ...${namespace}.include(base.a), color: tokens.c,
          animationName: frames, width: ${namespace}.firstThatWorks("1px", "auto") } });
        ${namespace}.props(base.a, runtime); ${namespace}.attrs(base.a, runtime);
        ${namespace}.firstThatWorks(runtime); ${namespace}.include(runtime);`,
    })),
    ...['@stylexjs/stylex', '@devup-ui/react/stylex'].map((source) => ({
      code: `import { create as build, types as t, firstThatWorks as first, include as inc,
        keyframes as frames } from "${source}";
        const fade = frames({ from: { opacity: 0 } });
        const base = build({ a: { color: "red" } });
        build({ a: { ...inc(base.a), color: t.color("red"),
          animationName: fade, width: first("1px", "auto") } });`,
    })),
    ...[
      'import sx from "@devup-ui/react/stylex"; sx.create({ a: { color: value } });',
      'import { default as sx } from "@devup-ui/react/stylex"; sx.create({ a: { color: value } });',
      'import sx from "@devup-ui/react"; sx.stylex.create({ a: { color: value } });',
      'import { positionTry as build } from "@devup-ui/react"; build({ a: { color: value } });',
      'import * as sx from "@devup-ui/react/compat/stylex"; sx.create({ a: { color: value } });',
      'import * as sx from "@stylexjs/stylex/extra"; sx.create({ a: { color: value } });',
      'import * as sx from "@devup-ui/react/stylex/extra"; sx.create({ a: { color: value } });',
      'import type * as sx from "@stylexjs/stylex"; sx.create({ a: { color: value } });',
      'import type { stylex as sx } from "@devup-ui/react"; sx.create({ a: { color: value } });',
      'import { type create as build } from "@devup-ui/react/stylex"; build({ a: { color: value } });',
      'import * as sx from "@stylexjs/stylex"; function f(sx) { sx.create({ a: { color: value } }); }',
      'import { create as build } from "@stylexjs/stylex"; function f(build) { build({ a: { color: value } }); }',
      'import { stylex } from "@devup-ui/react"; function f(stylex) { stylex.create({ a: { color: value } }); }',
      'import * as devup from "@devup-ui/react"; function f(devup) { devup.stylex.create({ a: { color: value } }); }',
      'function f(require) { const sx = require("@stylexjs/stylex"); sx.create({ a: { color: value } }); }',
      'const sx = require(source); sx.create({ a: { color: value } });',
      'const sx = require("@stylexjs/stylex", extra); sx.create({ a: { color: value } });',
      'const sx = require("@devup-ui/react/compat/stylex"); sx.create({ a: { color: value } });',
      'const { create: build = fallback } = require("@stylexjs/stylex"); build({ a: { color: value } });',
      'const { [key]: build } = require("@stylexjs/stylex"); build({ a: { color: value } });',
      'const [sx] = require("@stylexjs/stylex"); sx.create({ a: { color: value } });',
      'import * as sx from "@stylexjs/stylex"; let build = sx.create; build = other; build({ a: { color: value } });',
      'import * as sx from "@stylexjs/stylex"; sx.unknown({ a: { color: value } });',
      'import * as devup from "@devup-ui/react"; devup.getTheme(value); devup.styles({ color: value }); devup.positionTry({ color: value });',
      'const { getTheme = fallback, styles } = require("@devup-ui/react"); getTheme(value); styles({ color: value });',
      'const a = b; const b = a; a.create({ color: value });',
      'const a = 1; a.create({ color: value });',
      'function sx() {} sx.create({ color: value });',
      'import * as sx from "@stylexjs/stylex"; sx["create"]({ color: value }); sx?.create({ color: value });',
      'import * as sx from "@stylexjs/stylex"; sx.types.unknown({ color: value });',
      'const { ...sx } = require("@stylexjs/stylex"); sx.create({ color: value });',
      'import sx = require("@stylexjs/stylex"); sx.create({ color: value });',
      'const { 1: build } = require("@stylexjs/stylex"); build({ color: value });',
      'let sx; sx = other; sx.create({ color: value });',
    ].map((code) => ({ code: `let value = "red"; ${code}` })),
    { code: 'import { css } from "@devup-ui/react"; css({ color: "red" });' },
  ],
  invalid: [
    ...calls.flatMap(([setup, callee]) => [
      {
        code: `${setup} let value = "red"; ${callee}({ a: { color: value } });`,
        errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
      },
      {
        code: `${setup} function f(value) { return ${callee}({ a: { color: value } }); }`,
        errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
      },
    ]),
    ...[
      'import * as sx from "@devup-ui/react/stylex"; sx.create({ a: { color: sx.types.color(value) } });',
      'import * as sx from "@stylexjs/stylex"; sx.create({ a: { width: sx.firstThatWorks(value, "auto") } });',
      'import { create, types as t } from "@devup-ui/react/stylex"; create({ a: { color: t.color(value) } });',
      'import * as sx from "@stylexjs/stylex"; const build = (sx.create as Function); build({ color: value });',
      'import * as sx from "@stylexjs/stylex"; const build = sx.create satisfies Function; build({ color: value });',
      'import * as sx from "@stylexjs/stylex"; const build = sx.create!; build({ color: value });',
      'const sx = require("@stylexjs/stylex") as object; sx.create({ color: value });',
      'const sx = require("@stylexjs/stylex") satisfies object; sx.create({ color: value });',
      'const sx = require("@stylexjs/stylex")!; sx.create({ color: value });',
      'import { css } from "@devup-ui/react"; import * as sx from "@stylexjs/stylex"; function f(sx) { css({ color: sx.firstThatWorks("red") }); }',
      'import { css } from "@devup-ui/react"; import * as devup from "@devup-ui/react"; css({ color: devup.getTheme() });',
      'import { css } from "@devup-ui/react"; import * as sx from "@stylexjs/stylex"; const wrap = sx.types.color; css({ color: wrap("red") });',
    ].map((code) => ({
      code: `let value = "red"; ${code}`,
      errors: [{ messageId: 'cssUtilsLiteralOnly' as const }],
    })),
  ],
})
