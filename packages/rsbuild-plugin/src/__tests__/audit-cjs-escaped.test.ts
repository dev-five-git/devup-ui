import assert from 'node:assert/strict'

import { expect, it } from 'bun:test'

import { fixture, record, red, run } from './audit-fixture'

it.each([
  {
    name: 'hex-double-unicode-single',
    barrel: String.raw`exports.A=require("./\x41.js"); exports.B=require('.\u002f\u0042.js');`,
  },
  {
    name: 'slash-codepoint-single-hex-double',
    barrel: String.raw`exports.A=require('.\/\u{41}.js'); exports.B=require("./\x42.js");`,
  },
])(
  'reserves escaped physical leaves before genuine $name AB/BA transforms',
  ({ barrel }) => {
    // Given
    using fs = fixture()
    fs.put('devup.json', '{}')
    fs.put(
      'src/entry.js',
      'import {A,B} from "@devup-ui/widgets"; console.log(A,B);',
    )
    fs.put(
      'node_modules/@devup-ui/widgets/package.json',
      '{"main":"dist/index.js"}',
    )
    const directory = 'node_modules/@devup-ui/widgets/dist'
    fs.put(`${directory}/index.js`, barrel)
    fs.put(`${directory}/A.js`, red)
    fs.put(`${directory}/B.js`, red.replace('red', 'blue'))
    // When
    const ab = run(fs.root, 'barrel', 'AB')
    const ba = run(fs.root, 'barrel', 'BA')
    // Then
    expect(ab.ok).toBe(true)
    expect(ba.ok).toBe(true)
    expect(ab.css).toEqual(ba.css)
    assert(Array.isArray(ab.revisions) && Array.isArray(ba.revisions))
    const a: unknown = ab.revisions.at(-1)
    const b: unknown = ba.revisions.at(-1)
    record(a)
    record(b)
    expect(a.assets).toEqual(b.assets)
    expect(ab.count).toBe(0)
    expect(ba.count).toBe(0)
  },
  120000,
)
