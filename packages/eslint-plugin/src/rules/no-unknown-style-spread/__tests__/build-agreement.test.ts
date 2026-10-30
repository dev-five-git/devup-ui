import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import { Linter } from 'eslint'
import { parser } from 'typescript-eslint'

import {
  codeExtract,
  getCss,
  resetStateForTesting,
  setDebug,
  setModuleResolver,
} from '../../../../../../bindings/devup-ui-wasm/pkg/index.js'
import { noUnknownStyleSpread } from '../index'
import { agreementCases } from './agreement-cases'

type BuildOutcome =
  | { readonly kind: 'extracted'; readonly code: string; readonly css: string }
  | { readonly kind: 'refused'; readonly error: Error }

const root = join(import.meta.dir, 'fixtures')
const filename = join(root, 'case.tsx')
const knownFile = join(root, 'known.ts').replaceAll('\\', '/')
const knownCode = readFileSync(knownFile, 'utf8')

function build(source: string, singleCss: boolean): BuildOutcome {
  try {
    using output = codeExtract(
      filename.replaceAll('\\', '/'),
      source,
      '@devup-ui/react',
      'devup-ui',
      singleCss,
      false,
      true,
      {},
    )
    return {
      kind: 'extracted',
      code: output.code,
      css: singleCss ? getCss(null, false) : (output.css ?? ''),
    }
  } catch (error) {
    if (!(error instanceof Error)) throw error
    return { kind: 'refused', error }
  }
}

describe('real build and typed spread lint agreement', () => {
  beforeEach(() => {
    resetStateForTesting()
    setDebug(true)
    setModuleResolver((specifier: string) =>
      specifier === './known'
        ? { path: knownFile, code: knownCode }
        : undefined,
    )
  })
  afterEach(resetStateForTesting)

  for (const singleCss of [false, true]) {
    for (const entry of agreementCases) {
      it(`${entry.name} agrees when singleCss=${singleCss}`, () => {
        // Given the real fixture import and unchanged built WASM, with isolated build state.
        const source = `import { Box, type DevupProps } from '@devup-ui/react'; import { styles as knownImport } from './known';\n${entry.body}`
        // When extracting and linting the identical source through their real adapters.
        const outcome = build(source, singleCss)
        const messages = new Linter().verify(
          source,
          [
            {
              files: ['**/*.tsx'],
              languageOptions: {
                parser,
                parserOptions: { projectService: true, tsconfigRootDir: root },
              },
              plugins: {
                agreement: { rules: { spread: noUnknownStyleSpread } },
              },
              rules: { 'agreement/spread': 'error' },
            },
          ],
          { filename },
        )
        // Then retained/composed/refused build behavior and the exact lint result agree with the independent case contract.
        switch (entry.outcome) {
          case 'pass-through':
            expect(outcome).toMatchObject({
              kind: 'extracted',
              code: expect.stringMatching(/<div\s+\{\.\.\./),
              css: expect.not.stringMatching(/background:\s*red\b/),
            })
            break
          case 'composed':
            expect(outcome).toMatchObject({
              kind: 'extracted',
              code: expect.not.stringMatching(/<div\s+\{\.\.\./),
              css: expect.stringMatching(/background:\s*red\b/),
            })
            break
          case 'refused':
            expect(outcome).toMatchObject({
              kind: 'refused',
              error: expect.objectContaining({
                message: expect.stringMatching(/case\.tsx:\d+:\d+:/),
              }),
            })
            break
          default:
            throw new TypeError(
              `Unexpected agreement case: ${String(entry satisfies never)}`,
            )
        }
        expect(
          messages.map(({ ruleId, messageId }) => ({ ruleId, messageId })),
        ).toEqual(
          entry.reports === 1
            ? [{ ruleId: 'agreement/spread', messageId: 'unknownStyleSpread' }]
            : [],
        )
      }, 30000)
    }
  }
})
