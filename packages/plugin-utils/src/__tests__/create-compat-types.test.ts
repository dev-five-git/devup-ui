import { describe, expect, it } from 'bun:test'

import { createCompatTypes, mergeImportAliases } from '../types'

describe('createCompatTypes', () => {
  it('references an entry for every default alias plus stylex', () => {
    expect(createCompatTypes(mergeImportAliases())).toBe(
      [
        '/// <reference types="@devup-ui/react/compat/emotion" />',
        '/// <reference types="@devup-ui/react/compat/styled-components" />',
        '/// <reference types="@devup-ui/react/compat/stylex" />',
        '/// <reference types="@devup-ui/react/compat/vanilla-extract" />',
      ].join('\n') + '\n',
    )
  })

  it('omits a disabled alias so its own types keep winning', () => {
    expect(
      createCompatTypes(
        mergeImportAliases({
          '@emotion/react': false,
          '@emotion/styled': false,
          '@vanilla-extract/css': false,
        }),
      ),
    ).toBe(
      [
        '/// <reference types="@devup-ui/react/compat/styled-components" />',
        '/// <reference types="@devup-ui/react/compat/stylex" />',
      ].join('\n') + '\n',
    )
  })

  it('collapses both emotion packages onto one entry', () => {
    expect(
      createCompatTypes({
        '@emotion/react': null,
        '@emotion/styled': 'styled',
      }),
    ).toBe(
      [
        '/// <reference types="@devup-ui/react/compat/emotion" />',
        '/// <reference types="@devup-ui/react/compat/stylex" />',
      ].join('\n') + '\n',
    )
  })

  it('still emits stylex when nothing is aliased', () => {
    expect(createCompatTypes({})).toBe(
      '/// <reference types="@devup-ui/react/compat/stylex" />\n',
    )
  })

  it('ignores aliases with no compat entry', () => {
    expect(createCompatTypes({ 'some-lib': 'styled' })).toBe(
      '/// <reference types="@devup-ui/react/compat/stylex" />\n',
    )
  })
})
