import { RuleTester } from '@typescript-eslint/rule-tester'
import type { TSESLint } from '@typescript-eslint/utils'
import { describe, expect, it } from 'bun:test'

import { ImportStorage, isVanillaExtractFile } from '../import-storage'
import { styleValueSite } from '../style-position'

const ruleTester = new RuleTester({
  languageOptions: {
    ecmaVersion: 'latest',
    parserOptions: { ecmaFeatures: { jsx: true } },
  },
})

/** Runs `check` on what the file imports, as a rule collecting its imports sees it, failing the run when `check` throws */
function inspect(
  code: string,
  filename: string,
  check: (
    storage: ImportStorage,
    context: Readonly<TSESLint.RuleContext<string, []>>,
  ) => void,
  withContext = true,
) {
  const rule: TSESLint.RuleModule<'unused', []> = {
    meta: { schema: [], messages: { unused: 'unused' }, type: 'problem' },
    defaultOptions: [],
    create(context) {
      const storage = new ImportStorage(withContext ? context : undefined)
      return {
        ImportDeclaration(node) {
          storage.addImportByDeclaration(node)
        },
        'Program:exit'() {
          check(storage, context)
        },
      }
    },
  }
  ruleTester.run('inspect', rule, { valid: [{ code, filename }], invalid: [] })
}

describe('import sources', () => {
  inspect(
    [
      'import { css as a, Box as B, unknown as u } from "@devup-ui/react";',
      'import { css as c, keyframes as k, Global, jsx, "css" as d } from "@emotion/react";',
      'import styled, { css as e, createGlobalStyle } from "styled-components";',
      'import * as E from "@emotion/styled";',
      'import { style, globalStyle, other } from "@vanilla-extract/css";',
      'import { css as skipped } from "@emotion/css";',
    ].join('\n'),
    'page.tsx',
    (storage) => {
      expect(Object.fromEntries(storage.bindings())).toEqual({
        a: 'css',
        B: 'Box',
        u: 'unknown',
        c: 'css',
        k: 'keyframes',
        Global: 'Global',
        d: 'css',
        styled: 'styled',
        e: 'css',
        createGlobalStyle: 'createGlobalStyle',
        E: 'styled',
        style: 'css',
        globalStyle: 'globalCss',
      })
    },
  )

  inspect(
    [
      'import Devup from "@devup-ui/react";',
      'import * as Whole from "@devup-ui/react/compat";',
      'import * as Emotion from "@emotion/react";',
      'import styled from "@emotion/styled";',
    ].join('\n'),
    'page.tsx',
    (storage) => {
      expect(storage.isImportObject('Devup')).toBe(true)
      expect(storage.isImportObject('Whole')).toBe(true)
      expect(storage.isImportObject('Emotion')).toBe(false)
      expect(storage.importedName('styled')).toBe('styled')
      expect(storage.importedName('Emotion')).toBeUndefined()
    },
  )

  for (const filename of ['a.css.ts', 'a.css.js'])
    inspect(
      'import { style } from "@vanilla-extract/css";\nimport { css } from "@devup-ui/react";',
      filename,
      (storage) => {
        expect(storage.vanilla).toBe(true)
        expect(storage.bindings()).toEqual([])
      },
    )

  inspect(
    'import { css } from "@devup-ui/react";\nconst s = { w: [1, 1] };\ncss(s)',
    'page.tsx',
    (storage, context) => {
      const program = context.sourceCode.ast
      expect(storage.declaredVariables(program)).toEqual([])
      const declaration = program.body[1]
      if (declaration.type !== 'VariableDeclaration') throw new Error('shape')
      const object = declaration.declarations[0].init
      if (!object) throw new Error('shape')
      expect(styleValueSite(object, storage)).toBeNull()
    },
    false,
  )

  it('tells vanilla-extract stylesheets from other files', () => {
    expect(isVanillaExtractFile('a.css.tsx')).toBe(false)
    expect(isVanillaExtractFile('css.ts')).toBe(false)
  })
})
