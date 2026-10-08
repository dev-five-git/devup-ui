import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import * as reactModule from '@devup-ui/react'
import { afterAll, beforeAll, describe, expect, it, spyOn } from 'bun:test'

let globalCssSpy: ReturnType<typeof spyOn>

beforeAll(() => {
  globalCssSpy = spyOn(reactModule, 'globalCss').mockReturnValue(undefined)
})

afterAll(() => {
  globalCssSpy.mockRestore()
})

describe('reset-css', () => {
  it('should be defined', async () => {
    // Dynamic import to ensure spy is in place
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
