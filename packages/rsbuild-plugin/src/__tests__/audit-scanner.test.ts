import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { fixture, record, red, run } from './audit-fixture'

it('keeps actual native bytes equal when only a quoted fake import target exists', () => {
  // Given: ordinary double-quoted payload, never an actual import expression.
  using fs = fixture()
  fs.put('devup.json', '{}')
  const input = `${red}\nexport const snippet="import '../out/stale.js'"; console.log(snippet);`
  fs.put('src/entry.js', input)
  const cold = run(fs.root, 'cold')
  expect(cold.ok).toBe(true)
  fs.put('out/stale.js', red.replace('red', 'chartreuse'))
  // When
  const warm = run(fs.root, 'cold')
  // Then: source is unchanged and genuine native emission, not a projected name.
  expect(readFileSync(join(fs.root, 'src/entry.js'), 'utf8')).toBe(input)
  expect(warm.ok).toBe(true)
  assert(Array.isArray(cold.revisions) && Array.isArray(warm.revisions))
  const a: unknown = cold.revisions.at(-1)
  const b: unknown = warm.revisions.at(-1)
  record(a)
  record(b)
  expect(b.assets).toEqual(a.assets)
  expect(warm.css).toEqual(cold.css)
}, 120000)
