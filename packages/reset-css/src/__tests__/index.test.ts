import { readFileSync } from 'node:fs'
import { join } from 'node:path'

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

  it('emits the accessible-hidden rules of sanitize.css 13', async () => {
    const { codeExtract, getCss } =
      await import('../../../../bindings/devup-ui-wasm/pkg/index.js')
    codeExtract(
      'reset-css.ts',
      readFileSync(join(import.meta.dir, '../index.ts'), 'utf8'),
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    const css = getCss(null, false)
    // The debug build the tests run repeats declarations: compare the distinct ones
    const declarationsOf = (selector: string) => {
      const start = css.indexOf(`${selector}{`) + selector.length + 1
      return [...new Set(css.slice(start, css.indexOf('}', start)).split(';'))]
    }

    // [aria-hidden="false" i][hidden] shows what is hidden for sight to assistive technology,
    // and clips it away from sight until `:focus` shows it
    expect(declarationsOf(':where([aria-hidden=false i][hidden])')).toEqual([
      'display:initial',
    ])
    expect(
      declarationsOf(':where([aria-hidden=false i][hidden]:not(:focus))'),
    ).toEqual(['clip:rect(0,0,0,0)', 'position:absolute'])
  })
})
