import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { cssUtilsLiteralOnly } from '../index'

describe('css-utils-literal-only rule: libraries the build reads as Devup UI', () => {
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const filename = 'src/app/page.tsx'
  const emotion = `import { css, keyframes, Global } from "@emotion/react";\nimport styled from "@emotion/styled";\n`
  const styledComponents = `import styled, { css, keyframes, createGlobalStyle } from "styled-components";\n`
  const vanilla = `import { style, globalStyle, keyframes } from "@vanilla-extract/css";\n`
  const devup = `import { css, globalCss } from "@devup-ui/react";\n`
  const error = { messageId: 'cssUtilsLiteralOnly' as const }
  const missing = (name: string) => ({
    messageId: 'missingGlobal' as const,
    data: { name },
  })

  ruleTester.run('literal-only', cssUtilsLiteralOnly, {
    valid: [
      ...[
        [emotion, 'css({ color: "red", p: [1, 2] })'],
        [emotion, 'keyframes({ from: { opacity: 0 } })'],
        [emotion, 'css`color: ${"red"};`'],
        [
          emotion,
          'function Component() {\n  const fade = keyframes({ from: { opacity: 0 } });\n  return css({ animationName: fade })\n}',
        ],
        [
          emotion,
          'const LIMIT = 4;\n<Global styles={{ body: { p: LIMIT } }} />',
        ],
        [
          emotion,
          '<Global styles={{ body: { color: "red" } }} data-x={window.name} other={v} />',
        ],
        [emotion, '<Global styles />'],
        [emotion, '<Global styles="body {}" />'],
        [emotion, '<Global.Sub styles={{ color: v }} />'],
        [emotion, '<div styles={{ color: v }} />'],
        [emotion, 'styled("div", { color: window.name })'],
        [emotion, 'styled.div({ color: v })'],
        [styledComponents, 'css({ color: "red" })'],
        [styledComponents, 'keyframes({ from: { opacity: 0 } })'],
        [styledComponents, 'createGlobalStyle({ body: { color: "red" } })'],
        [styledComponents, 'createGlobalStyle`body { color: red; }`'],
        [styledComponents, 'styled.div({ color: v })'],
        [styledComponents, 'styled.div`color: ${(props) => props.color};`'],
        [vanilla, 'style({ color: "red" })'],
        [vanilla, 'globalStyle("body", { color: "red" })'],
        [devup, 'globalCss({ body: { color: "red" } })'],
        [
          `import { css } from "@emotion/css";\n`,
          'css({ color: window.name })',
        ],
        [`import { css } from "other";\n`, 'css({ color: window.name })'],
        [
          `import { jsx } from "@emotion/react";\n`,
          'jsx({ color: window.name })',
        ],
      ].map(([header, source]) => ({ code: header + source, filename })),
      ...[
        'export const root = style({ color: "red" })',
        'const size = (n) => `${n * 4}px`;\nexport const root = style({ width: size(4) })',
        'const widths = [1, 2].map((n) => n * 4);\nexport const root = style({ width: widths[0] })',
        'export const guarded = typeof window === "undefined" ? 1 : 2',
        'export const defined = typeof document !== "undefined" && typeof process !== "undefined"',
        'const window = { name: "a" };\nexport const root = style({ content: window.name })',
        'export const root = style({ content: String(Date.now()) })',
        'export const local = (setTimeout) => setTimeout',
        'export const x = { window: 1 }.window',
        'export const y = globalThis',
        'export const z = Math.PI + Symbol.length',
      ].map((source) => ({
        code: vanilla + source,
        filename: 'src/styles.css.ts',
      })),
      {
        code: 'export const x = window.name',
        filename: 'src/page.tsx',
      },
      {
        code: `${devup}css({ color: "red" })`,
        filename: 'src/styles.css.js',
      },
    ],
    invalid: [
      ...[
        [emotion, 'css({ color: window.name })'],
        [emotion, 'keyframes({ from: { opacity: window.ratio } })'],
        [emotion, 'let v = 1;\ncss({ color: v })'],
        [emotion, 'function Component(v) {\n  return css({ color: v })\n}'],
        [emotion, '<Global styles={{ body: { color: window.name } }} />'],
        [emotion, 'let v = 1;\n<Global styles={{ body: { p: v } }} />'],
        [
          emotion,
          'let v = 1;\n<Global styles={{ body: { p: v } }} data-x={window.name} />',
        ],
        [styledComponents, 'css({ color: window.name })'],
        [styledComponents, 'keyframes({ from: { opacity: window.ratio } })'],
        [
          styledComponents,
          'createGlobalStyle({ body: { color: window.name } })',
        ],
        [
          styledComponents,
          'let v = 1;\ncreateGlobalStyle`body { color: ${v}; }`',
        ],
        [vanilla, 'style({ color: window.name })'],
        [vanilla, 'let v = 1;\nglobalStyle("body", { color: v })'],
        [vanilla, 'let v = 1;\nkeyframes({ from: { opacity: v } })'],
        [devup, 'let v = 1;\nglobalCss({ body: { color: v } })'],
      ].map(([header, source]) => ({
        code: header + source,
        filename,
        errors: [error],
      })),
      {
        code:
          emotion +
          'function Component(v) {\n  const fade = keyframes({ from: { opacity: v } });\n  return css({ animationName: fade })\n}',
        filename,
        errors: [error, error],
      },
      ...[
        ['export const a = window.name', ['window']],
        [
          'export const a = [document.title, process.env.NODE_ENV, window.name]',
          ['document', 'process', 'window'],
        ],
        [
          'export const root = style({ color: window.name });\nexport const other = style({ color: window.theme })',
          ['window', 'window'],
        ],
        ['setTimeout(() => {}, 1)', ['setTimeout']],
        ['export const a = typeof window.name', ['window']],
        ['export const a = require("x")', ['require']],
        ['export const a = Buffer.from("a")', ['Buffer']],
        ['export const a = new URL("https://a.b")', ['URL']],
      ].map(([source, names]) => ({
        code: vanilla + (source as string),
        filename: 'src/styles.css.ts',
        errors: (names as string[]).map(missing),
      })),
      {
        code: 'export const a = [localStorage, navigator]',
        filename: 'src/styles.css.js',
        errors: [missing('localStorage'), missing('navigator')],
      },
    ],
  })
})
