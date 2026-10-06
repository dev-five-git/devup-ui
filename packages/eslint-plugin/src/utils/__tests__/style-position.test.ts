import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'

import { isPassThroughProp, SPECIAL_PROPERTIES } from '../style-position'

describe('style-position', () => {
  it('passes through exactly the props the build does', () => {
    const rust = readFileSync(
      join(
        import.meta.dir,
        '../../../../../libs/css/src/is_special_property.rs',
      ),
      'utf8',
    )
    const start = rust.indexOf('static SPECIAL_PROPERTIES')
    const names = [
      ...rust.slice(start, rust.indexOf('};', start)).matchAll(/"([^"]+)"/g),
    ].map((match) => match[1])
    expect([...SPECIAL_PROPERTIES].sort()).toEqual([...new Set(names)].sort())
  })

  it('reads every other prop as a style', () => {
    for (const name of [
      'onClick',
      'data-id',
      'aria-label',
      'className',
      'as',
      'props',
      'styleVars',
      'styleOrder',
    ])
      expect(isPassThroughProp(name)).toBe(true)
    for (const name of [
      'bg',
      'w',
      'items',
      '_hover',
      'selectors',
      'typography',
    ])
      expect(isPassThroughProp(name)).toBe(false)
  })
})
