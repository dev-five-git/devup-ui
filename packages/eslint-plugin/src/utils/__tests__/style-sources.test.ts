import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { noDuplicateValue } from '../../rules/no-duplicate-value'
import { noTypographyTokenPrefix } from '../../rules/no-typography-token-prefix'
import { noUselessResponsive } from '../../rules/no-useless-responsive'
import { noUselessTailingNulls } from '../../rules/no-useless-tailing-nulls'
import { preferMediaShorthand } from '../../rules/prefer-media-shorthand'
import { styleOrderRange } from '../../rules/style-order-range'

const ruleTester = new RuleTester({
  languageOptions: {
    ecmaVersion: 'latest',
    parserOptions: { ecmaFeatures: { jsx: true } },
  },
})

/** What a file imports from each package the build reads as Devup UI's */
const HEADERS = {
  devup: `import { css, globalCss, keyframes, styled, Box } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\n`,
  emotion: `import { css, keyframes, Global } from "@emotion/react";\nimport styled from "@emotion/styled";\nimport * as Namespaced from "@emotion/styled";\n`,
  styledComponents: `import styled, { css, keyframes, createGlobalStyle } from "styled-components";\n`,
  vanilla: `import { style, globalStyle, keyframes } from "@vanilla-extract/css";\n`,
}

type Header = keyof typeof HEADERS

const filename = 'src/app/page.tsx'
/** The libraries giving `css`, `keyframes` and a `styled` */
const LIBRARIES: Header[] = ['devup', 'emotion', 'styledComponents']

/** A source, what a rule makes of it and how many reports it gives, in each library whose header is listed */
type Case = [headers: Header[], source: string, fixed: string, count?: number]

function invalid<M extends string>(cases: Case[], messageId: M) {
  return cases.flatMap(([headers, source, fixed, count = 1]) =>
    headers.map((header) => ({
      code: HEADERS[header] + source,
      output: HEADERS[header] + fixed,
      filename,
      errors: Array.from({ length: count }, () => ({ messageId })),
    })),
  )
}

function valid(sources: [Header[], string][]) {
  return sources.flatMap(([headers, source]) =>
    headers.map((header) => ({ code: HEADERS[header] + source, filename })),
  )
}

/** What the build does not read as styles: other packages, and vanilla-extract stylesheets, which it runs as a whole */
function elsewhere(source: string) {
  return [
    { code: `import { jsx } from "@emotion/react";\n${source}`, filename },
    { code: `import styled from "other-package";\n${source}`, filename },
    { code: `import { css } from "@emotion/css";\n${source}`, filename },
    { code: `${HEADERS.vanilla}${source}`, filename: 'src/styles.css.ts' },
    { code: `${HEADERS.devup}${source}`, filename: 'src/styles.css.js' },
  ]
}

describe('style sources: no-duplicate-value', () => {
  const fixes: Case[] = [
    [LIBRARIES, 'css({ w: [1, 1] })', 'css({ w: [1, null] })'],
    [
      [...LIBRARIES, 'vanilla'],
      'keyframes({ from: { w: [1, 1] } })',
      'keyframes({ from: { w: [1, null] } })',
    ],
    [['vanilla'], 'style({ w: [1, 1] })', 'style({ w: [1, null] })'],
    [
      ['vanilla'],
      'globalStyle("body", { w: [1, 1] })',
      'globalStyle("body", { w: [1, null] })',
    ],
    [
      ['devup'],
      'globalCss({ body: { w: [1, 1] } })',
      'globalCss({ body: { w: [1, null] } })',
    ],
    [
      ['styledComponents'],
      'createGlobalStyle({ body: { w: [1, 1] } })',
      'createGlobalStyle({ body: { w: [1, null] } })',
    ],
    [
      ['emotion'],
      '<Global styles={{ body: { w: [1, 1] } }} data-x={{ w: [1, 1] }} />',
      '<Global styles={{ body: { w: [1, null] } }} data-x={{ w: [1, 1] }} />',
    ],
    [
      LIBRARIES,
      'styled("div", { w: [1, 1] })',
      'styled("div", { w: [1, null] })',
    ],
    [LIBRARIES, 'styled.div({ w: [1, 1] })', 'styled.div({ w: [1, null] })'],
    [
      LIBRARIES,
      'styled("div")({ w: [1, 1] })',
      'styled("div")({ w: [1, null] })',
    ],
    [
      LIBRARIES,
      'styled(Base)({ w: [3, 3] }, { p: [4, 4] })',
      'styled(Base)({ w: [3, null] }, { p: [4, null] })',
      2,
    ],
    [
      LIBRARIES,
      'styled.div.attrs({ w: [1, 1] })({ p: [3, 3] })',
      'styled.div.attrs({ w: [1, 1] })({ p: [3, null] })',
    ],
    [
      LIBRARIES,
      'styled.div.withConfig({ w: [1, 1] })({ p: [3, 3] })',
      'styled.div.withConfig({ w: [1, 1] })({ p: [3, null] })',
    ],
    [
      ['devup'],
      'Devup.styled.div({ w: [1, 1] })',
      'Devup.styled.div({ w: [1, null] })',
    ],
    [
      ['emotion'],
      'Namespaced.div({ w: [1, 1] })',
      'Namespaced.div({ w: [1, null] })',
    ],
    [
      ['devup'],
      'const s = { w: [1, 1] };\ncss(s)',
      'const s = { w: [1, null] };\ncss(s)',
    ],
    [
      ['devup'],
      'const arr = [1, 1];\n<Box w={arr} />',
      'const arr = [1, null];\n<Box w={arr} />',
    ],
    [
      ['devup'],
      'const inner = { w: [1, 1] };\nconst outer = { _hover: inner };\ncss(outer);\nstyled.div(outer)',
      'const inner = { w: [1, null] };\nconst outer = { _hover: inner };\ncss(outer);\nstyled.div(outer)',
    ],
    [
      ['devup'],
      'const s = { w: [1, 1] };\ncss(s);\n<Box {...s} />',
      'const s = { w: [1, null] };\ncss(s);\n<Box {...s} />',
    ],
  ]
  ruleTester.run('aliases and positions', noDuplicateValue, {
    valid: [
      ...valid([
        [['devup'], 'const s = { w: [1, 1] };\nconsole.log(s);\ncss(s)'],
        [['devup'], 'const s = { w: [1, 1] };\ncss(s);\nexport { s }'],
        [['devup'], 'export const s = { w: [1, 1] };\ncss(s)'],
        [['devup'], 'const s = { w: [1, 1] };\ncss(s);\ns.w = [2];'],
        [['devup'], 'const s = { w: [1, 1] };\ncss(s);\nexport const t = s.w'],
        [
          ['devup'],
          'function f() {\n  const s = { w: [1, 1] };\n  return css(s)\n}',
        ],
        [['devup'], 'let s = { w: [1, 1] };\ncss(s)'],
        [['devup'], 'const s = { w: [1, 1] };\ncss(s);\ns = {}'],
        [['devup'], 'const { w } = { w: [1, 1] };\ncss({ w })'],
        [['devup'], 'const unused = { w: [1, 1] }'],
        [['devup'], 'const a = { w: [1, 1], b };\nconst b = { c: a };\ncss(a)'],
        [['devup'], 'const a = [{ w: [1, 1] }, a];\ncss(a)'],
        [LIBRARIES, 'styled.div.attrs({ w: [1, 1] })'],
        [LIBRARIES, 'styled("div", { w: [1, 1] }, extra)'],
        [LIBRARIES, 'styled("div", base, { w: [1, 1] })'],
        [LIBRARIES, 'styled("div", [{ w: [1, 1] }])'],
        [LIBRARIES, 'styled["div"]({ w: [1, 1] })'],
        [LIBRARIES, 'styled`w: ${[1, 1]};`'],
        [LIBRARIES, 'styled.div`w: ${[1, 1]};`'],
        [LIBRARIES, 'other({ w: [1, 1] })'],
        [LIBRARIES, 'unknown.div({ w: [1, 1] })'],
        [LIBRARIES, 'styled()'],
        [
          ['emotion'],
          '<Global data-x={{ w: [1, 1] }} other={{ w: [1, 1] }} />',
        ],
        [['emotion'], '<Global styles />'],
        [['emotion'], '<Namespaced.Global styles={{ w: [1, 1] }} />'],
        [['emotion'], '<div styles={{ w: [1, 1] }} />'],
        [
          ['emotion'],
          'jsx({ w: [1, 1] });\n<ThemeProvider theme={{ w: [1, 1] }} />',
        ],
      ]),
      ...elsewhere('css({ w: [1, 1] });\nstyled.div({ w: [1, 1] })'),
    ],
    invalid: invalid(fixes, 'duplicateValue'),
  })
})

describe('style sources: no-useless-tailing-nulls', () => {
  ruleTester.run('aliases and positions', noUselessTailingNulls, {
    valid: [
      ...valid([[LIBRARIES, 'styled.div.attrs({ w: [1, null] })']]),
      ...elsewhere('css({ w: [1, null] });\nstyled.div({ w: [1, null] })'),
    ],
    invalid: invalid(
      [
        [LIBRARIES, 'css({ w: [1, null] })', 'css({ w: [1] })'],
        [['vanilla'], 'style({ w: [1, null] })', 'style({ w: [1] })'],
        [['vanilla'], 'style({ w: [1, 2, null] })', 'style({ w: [1, 2] })'],
        [
          LIBRARIES,
          'styled.div({ w: [1, 2, null] })',
          'styled.div({ w: [1, 2] })',
        ],
        [
          ['devup'],
          'const s = { w: [1, null] };\ncss(s)',
          'const s = { w: [1] };\ncss(s)',
        ],
        [
          ['emotion'],
          '<Global styles={{ body: { w: [1, null] } }} />',
          '<Global styles={{ body: { w: [1] } }} />',
        ],
      ],
      'uselessTailingNulls',
    ),
  })
})

describe('style sources: no-useless-responsive', () => {
  ruleTester.run('aliases and positions', noUselessResponsive, {
    valid: [
      ...valid([
        [['devup'], 'const s = { w: [[1], 2] };\ncss(s)'],
        [['devup'], 'const s = [[1], 2];\ncss(s)'],
        [['devup'], 'const s = { w: [1] };\nconsole.log(s);\ncss(s)'],
        [['devup'], 'const s = { w: [1] };\ncss(s)\nconst t = s.w'],
      ]),
      ...elsewhere('css({ w: [1] });\nstyled.div({ w: [1] })'),
    ],
    invalid: invalid(
      [
        [LIBRARIES, 'css({ w: [1] })', 'css({ w: 1 })'],
        [['vanilla'], 'style({ w: [1] })', 'style({ w: 1 })'],
        [LIBRARIES, 'styled.div({ w: [1] })', 'styled.div({ w: 1 })'],
        [
          ['devup'],
          'const s = { w: [1] };\ncss(s)',
          'const s = { w: 1 };\ncss(s)',
        ],
        [
          ['devup'],
          'const arr = [1];\n<Box w={arr} />',
          'const arr = 1;\n<Box w={arr} />',
        ],
        [
          ['emotion'],
          '<Global styles={{ body: { w: [1] } }} />',
          '<Global styles={{ body: { w: 1 } }} />',
        ],
        [
          ['devup'],
          'const s = { w: [1] };\ncss(cond ? s : {})',
          'const s = { w: 1 };\ncss(cond ? s : {})',
        ],
      ],
      'uselessResponsive',
    ),
  })
})

describe('style sources: no-typography-token-prefix', () => {
  ruleTester.run('aliases and positions', noTypographyTokenPrefix, {
    valid: [
      ...valid([
        [['emotion'], '<Global data-x={{ typography: "$a" }} />'],
        [['devup'], 'const s = { typography: "$a" };\nconsole.log(s)'],
        [LIBRARIES, 'styled.div.attrs({ typography: "$a" })'],
      ]),
      ...elsewhere(
        'css({ typography: "$a" });\nstyled.div({ typography: "$a" })',
      ),
    ],
    invalid: invalid(
      [
        [LIBRARIES, 'css({ typography: "$a" })', 'css({ typography: "a" })'],
        [
          ['vanilla'],
          'style({ typography: "$a" })',
          'style({ typography: "a" })',
        ],
        [
          LIBRARIES,
          'styled.div({ typography: "$a" })',
          'styled.div({ typography: "a" })',
        ],
        [
          ['emotion'],
          '<Global styles={{ body: { typography: "$a" } }} />',
          '<Global styles={{ body: { typography: "a" } }} />',
        ],
        [
          ['devup'],
          'const s = { typography: "$a" };\ncss(s)',
          'const s = { typography: "a" };\ncss(s)',
        ],
      ],
      'noTypographyTokenPrefix',
    ),
  })
})

describe('style sources: prefer-media-shorthand', () => {
  ruleTester.run('aliases and positions', preferMediaShorthand, {
    valid: [
      ...valid([
        [['emotion'], '<Global data-x={{ "@media print": { w: 1 } }} />'],
        [['devup'], 'const s = { "@media print": { w: 1 } };\nconsole.log(s)'],
      ]),
      ...elsewhere(
        'css({ "@media print": { w: 1 } });\nstyled.div({ "@media print": { w: 1 } })',
      ),
    ],
    invalid: invalid(
      [
        [
          LIBRARIES,
          'css({ "@media print": { w: 1 } })',
          'css({ _print: { w: 1 } })',
        ],
        [
          ['vanilla'],
          'style({ "@media print": { w: 1 } })',
          'style({ _print: { w: 1 } })',
        ],
        [
          LIBRARIES,
          'styled.div({ "@media print": { w: 1 } })',
          'styled.div({ _print: { w: 1 } })',
        ],
        [
          ['emotion'],
          '<Global styles={{ "@media print": { w: 1 } }} />',
          '<Global styles={{ _print: { w: 1 } }} />',
        ],
        [
          ['devup'],
          'const s = { "@media print": { w: 1 } };\ncss(s)',
          'const s = { _print: { w: 1 } };\ncss(s)',
        ],
      ],
      'preferMediaShorthand',
    ),
  })
})

describe('style sources: style-order-range', () => {
  ruleTester.run('aliases and positions', styleOrderRange, {
    valid: [
      ...valid([
        [LIBRARIES, 'styled.div.attrs({ styleOrder: 0 })'],
        [LIBRARIES, 'styled("div", { styleOrder: 1 })'],
        [['emotion'], '<Global styles={{ styleOrder: 0 }} />'],
      ]),
      ...elsewhere('css({ styleOrder: 0 });\nstyled.div({ styleOrder: 0 })'),
    ],
    invalid: [
      ...LIBRARIES.flatMap((header) =>
        [
          'css({ styleOrder: 0 })',
          'styled.div({ styleOrder: 300 })',
          'styled("div", { styleOrder: 0 })',
          'styled(Base)({ p: 1 }, { styleOrder: 0 })',
        ].map((source) => ({
          code: HEADERS[header] + source,
          filename,
          errors: [{ messageId: 'styleOrderRange' as const }],
        })),
      ),
      ...[
        [HEADERS.vanilla, 'style({ styleOrder: 0 })'],
        [HEADERS.styledComponents, 'createGlobalStyle({ styleOrder: 0 })'],
        [HEADERS.devup, 'globalCss({ styleOrder: 0 })'],
      ].map(([header, source]) => ({
        code: header + source,
        filename,
        errors: [{ messageId: 'styleOrderRange' as const }],
      })),
    ],
  })
})
