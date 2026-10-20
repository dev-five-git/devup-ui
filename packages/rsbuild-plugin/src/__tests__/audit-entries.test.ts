import assert from 'node:assert/strict'

import { expect, it } from 'bun:test'

import { fixture, record, red, run } from './audit-fixture'

it.each(['override-run', 'override-build'])(
  'reserves all known physical inputs before genuine %s AB/BA transforms',
  (mode: string) => {
    // Given
    using fs = fixture()
    fs.put('devup.json', '{}')
    fs.put(
      'src/entry.js',
      mode === 'barrel'
        ? 'import {A,B} from "@devup-ui/widgets"; console.log(A,B);'
        : 'console.log("old-source-entry");',
    )
    const directory =
      mode === 'barrel' ? 'node_modules/@devup-ui/widgets/dist' : 'outside'
    if (mode === 'barrel') {
      fs.put(
        'node_modules/@devup-ui/widgets/package.json',
        '{"main":"dist/index.js"}',
      )
      fs.put(
        `${directory}/index.js`,
        'exports.A=require("./A.js"); exports.B=require("./B.js");',
      )
    }
    fs.put(`${directory}/A.js`, red)
    fs.put(`${directory}/B.js`, red.replace('red', 'blue'))
    // When: only callback release order changes, never native entry representation.
    const ab = run(fs.root, mode, 'AB')
    const ba = run(fs.root, mode, 'BA')
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
