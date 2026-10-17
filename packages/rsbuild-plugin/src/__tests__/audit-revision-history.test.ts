import assert from 'node:assert/strict'

import { expect, it } from 'bun:test'

import { fixture, record, run, tokenFixture } from './audit-fixture'

it.each(['manager', 'revision'])(
  'retains prior allocator leaves and reserves blue before live %s revision extraction',
  (mode) => {
    // Given
    using fs = fixture()
    tokenFixture(fs)
    // When
    const result = run(fs.root, mode)
    // Then
    assert(Array.isArray(result.batches))
    expect(result.batches).toHaveLength(2)
    const first: unknown = result.batches[0]
    const second: unknown = result.batches[1]
    record(first)
    record(second)
    expect(second.rows).toEqual(first.rows)
    expect(second.count).toBe(0)
    expect(second.files).toBe(first.files)
    expect(second.css).toContain('color:red')
    expect(second.css).not.toContain('color:blue')
    assert(
      typeof first.classes === 'string' && typeof second.classes === 'string',
    )
    const initial: unknown = JSON.parse(first.classes)
    const revised: unknown = JSON.parse(second.classes)
    record(initial)
    record(revised)
    const oldGlobal = initial['']
    const newGlobal = revised['']
    record(oldGlobal)
    record(newGlobal)
    expect(oldGlobal).toEqual({ 'color-0-red--255': 0 })
    expect(newGlobal).toEqual({ 'color-0-red--255': 0, 'color-0-blue--255': 1 })
    assert(Array.isArray(result.extracts))
    for (const extract of result.extracts) {
      record(extract)
      expect(extract.before).toBe(0)
      expect(extract.after).toBe(0)
    }
  },
  120000,
)
