import { RuleTester } from '@typescript-eslint/rule-tester'
import { describe } from 'bun:test'

import { cssUtilsLiteralOnly } from '../index'

/** The build never calls a function given as a style value: it is a build error */
describe('css-utils-literal-only rule: functions as values', () => {
  const ruleTester = new RuleTester({
    languageOptions: {
      ecmaVersion: 'latest',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
  })
  const filename = 'src/app/page.tsx'
  const devup = `import { css, globalCss, keyframes, createGlobalStyle } from "@devup-ui/react";\nimport * as Devup from "@devup-ui/react";\n`
  const styledComponents = `import styled, { css, createGlobalStyle } from "styled-components";\n`
  const emotion = `import { css, Global } from "@emotion/react";\n`
  const stylex = `import * as stylex from "@stylexjs/stylex";\nimport { create } from "@stylexjs/stylex";\n`
  const error = { messageId: 'functionValue' as const }

  ruleTester.run('functions as values', cssUtilsLiteralOnly, {
    valid: [
      [devup, 'css({ color: (() => "red")() })'],
      [devup, 'css({ gap: [1, 2].map((n) => n * 4)[1] })'],
      [devup, 'css({ color: { a: () => "red" }.a })'],
      [devup, 'css(cond && (() => "a"))'],
      [devup, 'css((() => "red")())'],
      [devup, 'const f = () => "red";\nglobalCss({ body: { color: f() } })'],
      [devup, 'css`color: ${(() => "red")()};`'],
      [devup, 'other({ color: () => "red" })'],
      [stylex, 'stylex.create({ dyn: (color) => ({ color }) })'],
      [stylex, 'create({ dyn: (color) => ({ color }) })'],
      [
        styledComponents,
        'styled.div`color: ${(p) => p.color};`\nstyled.div({ color: (p) => p.color })',
      ],
      [
        emotion,
        '<Global styles={{ body: { color: [1, 2].map((n) => n)[0] } }} />',
      ],
    ].map(([header, code]) => ({ code: header + code, filename })),
    invalid: [
      [devup, 'css({ color: () => "red" })'],
      [devup, 'css({ _hover: { color: cond ? "blue" : () => "red" } })'],
      [devup, 'css({ color: [() => "red", "blue"] })'],
      [devup, 'css({ color: (() => "red") as string })'],
      [devup, 'css`color: ${() => "red"};`'],
      [devup, 'globalCss(() => "red")'],
      [devup, 'globalCss`body { color: ${function () { return "red" }}; }`'],
      [devup, 'keyframes({ from: { opacity: () => 1 } })'],
      [devup, 'createGlobalStyle`body { color: ${(p) => p.c}; }`'],
      [devup, 'Devup.css({ color: () => "red" })'],
      [styledComponents, 'createGlobalStyle`body { color: ${(p) => p.c}; }`'],
      [styledComponents, 'css`color: ${(p) => p.c};`'],
      [styledComponents, 'styled.div`${css`color: ${(p) => p.c};`}`'],
      [emotion, 'css({ color: () => "red" })'],
      [emotion, '<Global styles={{ body: { color: () => "red" } }} />'],
    ].map(([header, code]) => ({
      code: header + code,
      filename,
      errors: [error],
    })),
  })
})
