import { describe, expect, it } from 'bun:test'

import { createLocator, decodeMappings, remapLocations } from '../source-map'

describe('decodeMappings', () => {
  it('decodes relative segments, skipping those without a source', () => {
    // Line 1: column 0 -> 1:0, column 4 -> 1:2; line 3: column 2 -> 2:0
    // (a negative column delta), column 33 -> 18:5 (multi-digit values).
    // `C` names no source; the empty group and line are skipped.
    expect(decodeMappings('AAAA,IAAE,C;;EACF,+BAgBK,')).toEqual([
      { line: 1, column: 0, source: { line: 1, column: 0 } },
      { line: 1, column: 4, source: { line: 1, column: 2 } },
      { line: 3, column: 2, source: { line: 2, column: 0 } },
      { line: 3, column: 33, source: { line: 18, column: 5 } },
    ])
  })
})

describe('createLocator', () => {
  const locate = createLocator(decodeMappings('AAAA,IAAE;;EACF'))

  it('moves a place by its distance from the segment on its line', () => {
    expect(locate({ line: 1, column: 6 })).toEqual({ line: 1, column: 4 })
    expect(locate({ line: 3, column: 9 })).toEqual({ line: 2, column: 7 })
  })

  it('places inserted lines at the segment before them', () => {
    expect(locate({ line: 2, column: 5 })).toEqual({ line: 1, column: 2 })
  })

  it('leaves a place before the first segment', () => {
    expect(createLocator(decodeMappings(';AAAA'))({ line: 1, column: 0 })).toBe(
      undefined,
    )
  })
})

describe('remapLocations', () => {
  it('moves every location of the AST once', () => {
    const shared = { line: 3, column: 2 }
    const comment = { type: 'CommentLine', loc: { start: shared, end: shared } }
    const program: Record<string, unknown> = {
      type: 'Program',
      loc: {
        start: { line: 1, column: 0 },
        end: { line: 3, column: 4 },
        filename: 'App.tsx',
      },
      body: [
        { type: 'Inserted', loc: { start: { line: 2, column: 0 } } },
        { type: 'Partial', loc: { start: { line: 'x' }, end: null } },
        { type: 'Plain', extra: null, value: 1 },
      ],
      comments: [comment],
      leadingComments: [comment],
    }
    program.self = program
    remapLocations({ type: 'File', program }, '{"mappings":";AAAA;EACA"}')
    expect(program.loc).toEqual({
      start: { line: 1, column: 0 },
      end: { line: 2, column: 2 },
      filename: 'App.tsx',
    })
    expect(program.body).toEqual([
      { type: 'Inserted', loc: { start: { line: 1, column: 0 } } },
      { type: 'Partial', loc: { start: { line: 'x' }, end: null } },
      { type: 'Plain', extra: null, value: 1 },
    ])
    expect(shared).toEqual({ line: 2, column: 0 })
  })

  it('accepts a missing AST', () => {
    expect(() => remapLocations(null, '{"mappings":""}')).not.toThrow()
  })
})
