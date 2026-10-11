import { readFileSync } from 'node:fs'

import { describe, expect, it } from 'bun:test'

import {
  codeExtractWithoutSourceMap,
  getCss,
} from '../../../../bindings/devup-ui-wasm/pkg'

describe('reset-css', () => {
  it('extracts reset rules without a runtime styling call', async () => {
    const source = readFileSync(new URL('../index.ts', import.meta.url), 'utf8')
    using output = codeExtractWithoutSourceMap(
      'reset-css.test.ts',
      source,
      '@devup-ui/react',
      '@devup-ui/react',
      true,
      false,
      false,
      {},
    )
    expect(output.code).not.toContain('globalCss')
    const css = getCss(null, false)
    expect(css).toContain(':where(body){margin:0}')
    expect(css).toContain('box-sizing:border-box')
    expect(css).toContain(':where(dialog:not([open])){display:none}')
    const { resetCss } = await import('../index')
    expect(resetCss).toBeInstanceOf(Function)
    expect(resetCss()).toBeUndefined()
  })
})
