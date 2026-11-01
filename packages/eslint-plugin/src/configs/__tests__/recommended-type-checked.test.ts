import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'
import { Linter } from 'eslint'
import { parser } from 'typescript-eslint'

import { StyleSchemaError } from '../../rules/no-unknown-style-spread/style-capability'
import recommended from '../recommended'
import { recommendedTypeChecked } from '../recommended-type-checked'

const root = join(
  import.meta.dir,
  '../../rules/no-unknown-style-spread/__tests__/fixtures',
)
const filename = join(root, 'case.tsx')
const code =
  'import { Box, type DevupProps } from "@devup-ui/react"; function f(props: DevupProps) { return <Box {...props} /> }'
const project = {
  languageOptions: {
    parserOptions: { projectService: true, tsconfigRootDir: root },
  },
}

describe('recommendedTypeChecked', () => {
  for (const configs of [
    recommendedTypeChecked,
    [...recommended, ...recommendedTypeChecked],
    [...recommendedTypeChecked, ...recommendedTypeChecked],
  ]) {
    it('reports runtime styles when typed configurations compose', () => {
      // Given a real TS project and composed presets.
      const linter = new Linter()
      // When linting through ESLint's flat-config surface.
      const messages = linter.verify(code, [...configs, project], { filename })
      // Then the typed rule reports once, without plugin redefinition.
      expect(
        messages.map(({ ruleId, messageId }) => ({ ruleId, messageId })),
      ).toEqual([
        {
          ruleId: '@devup-ui/type-checked/no-unknown-style-spread',
          messageId: 'unknownStyleSpread',
        },
      ])
    }, 30000)
  }

  it('keeps syntax-only linting when only recommended is enabled', () => {
    // Given the old preset without a program.
    const linter = new Linter()
    // When linting the same runtime spread with syntax-only TS parsing.
    const messages = linter.verify(
      code,
      [...recommended, { files: ['**/*.tsx'], languageOptions: { parser } }],
      { filename },
    )
    // Then no typed rule is enabled.
    expect(messages).toEqual([])
  })

  it('fails configuration when typed services are missing', () => {
    // Given the typed rule enabled without project/projectService.
    const linter = new Linter()
    // When linting, then parser-services acquisition must fail even for an empty file.
    expect(() =>
      linter.verify('export {}', recommendedTypeChecked, { filename }),
    ).toThrow(Error)
  })

  it('fails resolution when a recognized primitive has no declared schema', () => {
    // Given an invalid primitive import from the real compatibility entry.
    const source =
      'import { Box } from "@devup-ui/react/compat"; function f(props: unknown) { return <Box {...props} /> }'
    // When linting, then no empty style schema may silently pass.
    expect(() =>
      new Linter().verify(source, [...recommendedTypeChecked, project], {
        filename,
      }),
    ).toThrow(StyleSchemaError)
  }, 30000)
})
