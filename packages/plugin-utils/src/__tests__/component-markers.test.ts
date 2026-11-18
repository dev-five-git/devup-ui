import { beforeEach, describe, expect, it } from 'bun:test'

import {
  applyMarkerEdits,
  ComponentMarkerLedger,
  markerEditMap,
  markerEdits,
  markerLikeTokens,
  markerWarning,
  resetMarkerProcessState,
  unscannedCause,
} from '../component-markers'

beforeEach(() => {
  resetMarkerProcessState()
})

function transform(
  id: string,
  given: string[],
  selected: string[] = [],
  input = '',
) {
  return { id, input, givenMarkers: given, selectedMarkers: selected }
}

describe('ComponentMarkerLedger', () => {
  it('keeps every marker until a scan reports its selections', () => {
    // Given
    const ledger = new ComponentMarkerLedger()
    ledger.record(transform('/a.tsx', ['a--a']))
    // When
    const plan = ledger.plan()
    // Then
    expect(plan).toEqual({ keep: { kind: 'no-scan' } })
    ledger.scanned(undefined, [])
    expect(ledger.plan()).toEqual({ keep: { kind: 'no-scan' } })
  })

  it('keeps markers when a transform selects one the scan missed', () => {
    // Given
    const ledger = new ComponentMarkerLedger()
    ledger.scanned(['a--b'], ['/a.tsx', '/b.tsx'])
    ledger.record(transform('/a.tsx', ['a--a', 'a--b', 'a--c', 'a--d']))
    ledger.record(transform('/b.tsx', ['b--a'], ['a--c'], 'className="a--d"'))
    // When
    const plan = ledger.plan()
    // Then
    expect(plan).toEqual({ keep: expect.anything() })
  })

  it('names the modules the scan did not see select a marker', () => {
    // Given
    const ledger = new ComponentMarkerLedger()
    ledger.scanned(['a--b'], ['/a.tsx', '/c.tsx'])
    ledger.record(transform('/a.tsx', ['a--a', 'a--b']))
    ledger.record(transform('/z.tsx', [], ['a--a']))
    ledger.record({ ...transform('/v.tsx', [], ['a--a']), cause: 'virtual-module' })
    ledger.record(transform('/c.tsx', [], ['a--b', 'a--c']))
    // When
    const plan = ledger.plan()
    // Then
    expect(plan).toEqual({
      keep: {
        kind: 'unscanned',
        modules: [
          { id: '/c.tsx', cause: 'scan-failed', markers: ['a--c'] },
          { id: '/v.tsx', cause: 'virtual-module', markers: ['a--a'] },
          { id: '/z.tsx', cause: 'outside-scan', markers: ['a--a'] },
        ],
      },
    })
    // A later compilation of the same process keeps its markers too
    const later = new ComponentMarkerLedger()
    later.scanned([], [])
    expect(later.plan()).toEqual({ keep: { kind: 'earlier-compilation' } })
  })

  it('strips what is given and selected by no one, keeping written tokens', () => {
    // Given
    const ledger = new ComponentMarkerLedger()
    ledger.scanned(['a--b'], ['/a.tsx', '/b.tsx'])
    ledger.record(transform('/a.tsx', ['a--a', 'a--b', 'a--c', 'a--d']))
    ledger.record(transform('/b.tsx', ['b--a'], ['a--b'], 'className="a--d"'))
    ledger.record(transform('/b.tsx', ['b--a'], ['a--b'], 'className="a--d"'))
    // When
    const plan = ledger.plan()
    // Then
    expect(plan).toEqual({ strip: new Set(['a--a', 'a--c', 'b--a']) })
    ledger.clear()
    expect(ledger.plan()).toEqual({ strip: new Set() })
  })
})

describe('markerWarning', () => {
  it('says nothing for reasons that are not missing certainty', () => {
    expect(markerWarning({ kind: 'no-scan' })).toBeUndefined()
    expect(markerWarning({ kind: 'watch' })).toBeUndefined()
    expect(markerWarning({ kind: 'earlier-compilation' })).toBeUndefined()
  })

  it('lists each unscanned module with its cause and markers', () => {
    // Given
    const reason = {
      kind: 'unscanned' as const,
      modules: [
        { id: 'virtual:x', cause: 'virtual-module' as const, markers: ['a--a'] },
        { id: '/a.mdx', cause: 'compiled-source' as const, markers: ['a--b', 'a--c'] },
        { id: '/ctx.tsx', cause: 'context-module' as const, markers: ['a--d'] },
        { id: '/out.tsx', cause: 'outside-scan' as const, markers: ['a--e'] },
        { id: '/fail.tsx', cause: 'scan-failed' as const, markers: ['a--f'] },
      ],
    }
    // When
    const warning = markerWarning(reason)
    // Then
    expect(warning).toMatchSnapshot()
  })

  it('says when an earlier compilation already dropped a marker', () => {
    // Given
    const first = new ComponentMarkerLedger()
    first.scanned([], ['/a.tsx'])
    first.record(transform('/a.tsx', ['a--a']))
    expect(first.plan()).toEqual({ strip: new Set(['a--a']) })
    const second = new ComponentMarkerLedger()
    second.scanned([], [])
    second.record(transform('/late.tsx', [], ['a--a']))
    const plan = second.plan()
    // When
    const warning = 'keep' in plan ? markerWarning(plan.keep) : undefined
    // Then
    expect(warning).toContain('already dropped a--a')
  })
})

describe('unscannedCause', () => {
  it.each([
    ['\0virtual', 'x', 'x', 'virtual-module'],
    ['virtual:routes', 'x', 'x', 'virtual-module'],
    ['/gone.tsx', 'x', undefined, 'virtual-module'],
    ['/a.mdx', 'x', 'x', 'compiled-source'],
    ['/a.tsx', 'compiled', 'source', 'compiled-source'],
    ['/a.tsx?query', 'x', 'x', 'outside-scan'],
    ['/a.tsx', 'x', 'x', 'outside-scan'],
  ] as const)('reads %s', (id, input, onDisk, cause) => {
    expect(unscannedCause(id, input, onDisk)).toBe(cause)
  })
})

describe('markerLikeTokens', () => {
  it('finds tokens written like markers', () => {
    expect(markerLikeTokens('a b--c "d--e" `--f` x')).toEqual(
      new Set(['b--c', 'd--e']),
    )
    expect(markerLikeTokens('')).toEqual(new Set())
  })
})

describe('markerEdits', () => {
  const strip = new Set(['a--a', 'a--b'])

  it.each([
    ['"a--a b"', '"b"'],
    ['"b a--a"', '"b"'],
    ['"a--a"', '""'],
    ['`a--a b ${c}`', '`b ${c}`'],
    ['`a--a${c}`', '`${c}`'],
    ["'a--b c a--a'", "'c'"],
    ['"xa--a b"', '"xa--a b"'],
    ['"a--ax b"', '"a--ax b"'],
    ['"a--c b"', '"a--c b"'],
    ['i--a', 'i--a'],
  ])('rewrites %s', (code, expected) => {
    expect(applyMarkerEdits(code, markerEdits(code, strip))).toBe(expected)
  })

  it('makes no edits without markers to drop', () => {
    expect(markerEdits('"a--a"', new Set())).toEqual([])
  })
})

const BASE64 =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'

/** Each segment as `[generated line, column, original line, column]` */
function decode(mappings: string): [number, number, number, number][] {
  const result: [number, number, number, number][] = []
  let fields: number[] = [0, 0, 0, 0]
  for (const [generatedLine, group] of mappings.split(';').entries()) {
    fields[0] = 0
    for (const segment of group ? group.split(',') : []) {
      const values: number[] = []
      let value = 0
      let shift = 0
      for (const char of segment) {
        const digit = BASE64.indexOf(char)
        value += (digit & 31) << shift
        if (digit & 32) shift += 5
        else {
          values.push(value & 1 ? -(value >>> 1) : value >>> 1)
          value = 0
          shift = 0
        }
      }
      fields = fields.map((field, index) => field + (values[index] ?? 0))
      result.push([
        generatedLine,
        fields[0] ?? 0,
        fields[2] ?? 0,
        fields[3] ?? 0,
      ])
    }
  }
  return result
}

describe('markerEditMap', () => {
  it('maps each word of the rewritten code back to where it was', () => {
    // Given
    const code = 'const x = "a--a b";\nf(`a--b c ${d}`, -1);'
    const edits = markerEdits(code, strip())
    const rewritten = applyMarkerEdits(code, edits)
    // When
    const map = markerEditMap(code, edits)
    // Then
    expect(rewritten).toBe('const x = "b";\nf(`c ${d}`, -1);')
    const lines = rewritten.split('\n')
    const original = code.split('\n')
    const segments = decode(map.mappings)
    let words = 0
    for (const [generatedLine, column, line, originalColumn] of segments) {
      expect(line).toBe(generatedLine)
      const word = /^[\w$]+/.exec(lines[generatedLine]?.slice(column) ?? '')
      if (!word) continue
      words++
      expect(original[line]?.slice(originalColumn).startsWith(word[0])).toBe(
        true,
      )
    }
    expect(words).toBe(8)
    expect(map.mappings.split(';')).toHaveLength(2)
    expect(map.version).toBe(3)
  })

  function strip() {
    return new Set(['a--a', 'a--b'])
  }
})
