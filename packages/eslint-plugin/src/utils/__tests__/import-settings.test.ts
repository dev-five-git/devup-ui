import { RuleTester } from '@typescript-eslint/rule-tester'
import type { TSESLint } from '@typescript-eslint/utils'
import { describe, expect, it } from 'bun:test'

import { cssUtilsLiteralOnly } from '../../rules/css-utils-literal-only'
import { noUselessResponsive } from '../../rules/no-useless-responsive'
import { ImportStorage, importConfig, SETTINGS_KEY } from '../import-storage'

const ruleTester = new RuleTester({
  languageOptions: {
    ecmaVersion: 'latest',
    parserOptions: { ecmaFeatures: { jsx: true } },
  },
})

const settings = {
  [SETTINGS_KEY]: {
    package: '@acme/ui',
    importAliases: {
      'acme-styled': 'styled',
      'acme-css': true,
      'acme-other': 'notAnExport',
      '@emotion/styled': false,
      'styled-components': true,
    },
  },
}

/** Runs `check` on what the file imports under `settings`, as a rule collecting its imports sees it */
function inspect(code: string, check: (storage: ImportStorage) => void) {
  const rule: TSESLint.RuleModule<'unused', []> = {
    meta: { schema: [], messages: { unused: 'unused' }, type: 'problem' },
    defaultOptions: [],
    create(context) {
      const storage = new ImportStorage(context)
      return {
        ImportDeclaration(node) {
          storage.addImportByDeclaration(node)
        },
        'Program:exit'() {
          check(storage)
        },
      }
    },
  }
  ruleTester.run('inspect', rule, {
    valid: [{ code, filename: 'page.tsx', settings }],
    invalid: [],
  })
}

describe('import settings', () => {
  inspect(
    [
      'import s from "acme-styled";',
      'import * as S from "acme-styled";',
      'import d, { css as c } from "acme-css";',
      'import o from "acme-other";',
      'import e, { css as ec } from "@emotion/styled";',
      'import f, { keyframes as k } from "styled-components";',
      'import { Box as B } from "@acme/ui";',
      'import * as U from "@acme/ui/compat";',
      'import { css as x } from "@devup-ui/react";',
      'import { css as g } from "@emotion/react";',
    ].join('\n'),
    (storage) => {
      expect(Object.fromEntries(storage.bindings())).toEqual({
        s: 'styled',
        S: 'styled',
        c: 'css',
        k: 'keyframes',
        B: 'Box',
        g: 'css',
      })
      expect(storage.isImportObject('U')).toBe(true)
    },
  )

  it('reads the defaults without settings', () => {
    for (const shared of [undefined, {}, { [SETTINGS_KEY]: {} }]) {
      const config = importConfig(shared)
      expect(config.package).toBe('@devup-ui/react')
      expect(config.aliases.get('styled-components')).toBe('styled')
      expect(config.aliases.get('@emotion/react')).toBeNull()
    }
  })

  it('refuses settings the build plugin would not take', () => {
    for (const [devup, path] of [
      [1, ''],
      [{ package: 1 }, '.package'],
      [{ package: '' }, '.package'],
      [{ importAliases: [] }, '.importAliases'],
      [{ importAliases: { a: 1 } }, '.importAliases["a"]'],
      [{ importAliases: { a: '' } }, '.importAliases["a"]'],
    ] as const)
      expect(() => importConfig({ [SETTINGS_KEY]: devup })).toThrow(
        `settings["@devup-ui"]${path} must be`,
      )
  })

  ruleTester.run('configured aliases', noUselessResponsive, {
    valid: [
      {
        code: 'import styled from "@emotion/styled";\nstyled.div({ w: [1] })',
        filename: 'page.tsx',
        settings,
      },
    ],
    invalid: [
      {
        code: 'import styled from "acme-styled";\nstyled.div({ w: [1] })',
        output: 'import styled from "acme-styled";\nstyled.div({ w: 1 })',
        filename: 'page.tsx',
        settings,
        errors: [{ messageId: 'uselessResponsive' }],
      },
      {
        code: 'import { Box } from "@acme/ui";\n<Box w={[1]} />',
        output: 'import { Box } from "@acme/ui";\n<Box w={1} />',
        filename: 'page.tsx',
        settings,
        errors: [{ messageId: 'uselessResponsive' }],
      },
    ],
  })

  ruleTester.run('configured aliases', cssUtilsLiteralOnly, {
    valid: [
      {
        code: 'import { css } from "@devup-ui/react";\ncss({ color: window.name })',
        filename: 'page.tsx',
        settings,
      },
    ],
    invalid: [
      {
        code: 'import { css } from "acme-css";\ncss({ color: window.name })',
        filename: 'page.tsx',
        settings,
        errors: [{ messageId: 'cssUtilsLiteralOnly' }],
      },
    ],
  })
})
