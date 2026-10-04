import { describe, expect, it } from 'bun:test'

import { remapMdxError } from '../index'

const filename = 'C:\\docs\\page.mdx'
const map = {
  version: 3,
  sources: ['page.mdx', 'other.mdx'],
  names: ['Box'],
  mappings: ';EAIEA,KCEG;ADFH',
}

describe('MDX error locations', () => {
  it.each([map, JSON.stringify(map)])(
    'maps multiline Windows locations with real VLQ deltas',
    (input) => {
      const error = new Error(
        `${filename}:2:3: first\n${filename}:2:9: second\n${filename}:3:1: third\nother.ts:2:3: unchanged\nnote ${filename}:2:3: unchanged`,
      )
      const result = remapMdxError(error, filename, input)
      expect(result.message).toBe(
        'page.mdx:5:3: first\nother.mdx:7:6: second\npage.mdx:5:3: third\nother.ts:2:3: unchanged\nnote C:\\docs\\page.mdx:2:3: unchanged',
      )
      expect(result.cause).toBe(error)
    },
  )
  it.each([undefined, null])(
    'labels compiled coordinates immediately after the column when no map exists',
    (input) => {
      expect(
        remapMdxError(
          `${filename}:2:3: failure\n${filename}:4:5 details`,
          filename,
          input,
        ).message,
      ).toBe(
        `${filename}:2:3 (in compiled MDX): failure\n${filename}:4:5 (in compiled MDX) details`,
      )
    },
  )
  it('does not change a message without a matching location', () => {
    expect(remapMdxError(`${filename}:not a location`, filename).message).toBe(
      `${filename}:not a location`,
    )
  })
  it('labels gaps, before-first segments, unmapped segments and missing lines', () => {
    const input = {
      version: 3,
      sources: ['page.mdx'],
      mappings: ';EAAA,E;AAAA;;',
    }
    const text = [1, 2, 4, 8]
      .map((line) => `${filename}:${line}:1: missing`)
      .concat(`${filename}:2:5: gap`)
      .join('\n')
    expect(remapMdxError(text, filename, input).message).toBe(
      text
        .replace(/:(\d+):1:/g, ':$1:1 (in compiled MDX):')
        .replace(':2:5:', ':2:5 (in compiled MDX):'),
    )
  })
  it.each([
    ['', 'page.mdx', 'page.mdx'],
    ['/docs', 'page.mdx', '/docs/page.mdx'],
    ['C:\\docs', 'page.mdx', 'C:\\docs\\page.mdx'],
    [
      'https://example.com/docs/',
      '../page.mdx',
      'https://example.com/page.mdx',
    ],
    ['/docs', '/absolute.mdx', '/absolute.mdx'],
    ['/docs', 'C:\\absolute.mdx', 'C:\\absolute.mdx'],
    ['/docs', 'webpack:///page.mdx', 'webpack:///page.mdx'],
  ])('resolves sourceRoot %s and source %s', (sourceRoot, source, expected) => {
    expect(
      remapMdxError(`${filename}:1:1`, filename, {
        version: 3,
        mappings: 'AAAA',
        sources: [source],
        sourceRoot,
      }).message,
    ).toBe(`${expected}:1:1`)
  })
  it('decodes multi-digit VLQ fields and chooses the preceding mapping without inventing column offsets', () => {
    expect(
      remapMdxError(`${filename}:1:30`, filename, {
        version: 3,
        sources: ['page.mdx'],
        mappings: 'gBAgBgB',
      }).message,
    ).toBe('page.mdx:17:17')
  })
  it('tracks name deltas across segments and preserves the original error cause chain', () => {
    const cause = new TypeError('original')
    const error = new Error(`${filename}:1:3`, { cause })
    const result = remapMdxError(error, filename, {
      version: 3,
      sources: ['page.mdx'],
      names: ['a', 'b'],
      mappings: 'AAAAC,EAAAD',
    })
    expect(result.message).toBe('page.mdx:1:1')
    expect(result.cause).toBe(error)
    expect(error.cause).toBe(cause)
  })
  it.each([
    '{',
    'null',
    '[]',
    '{}',
    { version: 2, sections: [] },
    { ...map, version: 2 },
    { ...map, sources: [null] },
    { ...map, names: [1] },
    { ...map, sourceRoot: 1 },
    ...[
      '?',
      'g',
      'ggggggggggggA',
      '////////////A',
      'AA',
      'AAA',
      'AAAAAA',
      ',',
      'D',
      'EAAA,D',
      'ACAA',
      'ADAA',
      'AADA',
      'AAAD',
      'AAAAC',
      'AAAAD',
    ].map((mappings) => ({ ...map, sources: ['page.mdx'], mappings })),
  ])(
    'diagnoses invalid or unsupported maps with location and cause: %j',
    (input) => {
      let caught: unknown
      try {
        remapMdxError('failure', filename, input)
      } catch (error) {
        caught = error
      }
      expect(caught).toBeInstanceOf(Error)
      if (!(caught instanceof Error)) throw caught
      expect(caught.message).toStartWith(
        `${filename}:1:1: Invalid MDX source map:`,
      )
      expect(caught.cause).toBeInstanceOf(Error)
    },
  )
  it.each(['0:1', '1:0', '9007199254740992:1', '1:9007199254740992'])(
    'diagnoses invalid extractor coordinates %s',
    (location) => {
      expect(() => remapMdxError(`${filename}:${location}`, filename)).toThrow(
        `${filename}:1:1:`,
      )
    },
  )

  it('remaps indexed maps across offsets, gaps and nested sections', () => {
    const input = {
      version: 3,
      sections: [
        {
          offset: { line: 0, column: 2 },
          map: { version: 3, sources: ['first.mdx'], mappings: 'AAAA;AACE' },
        },
        {
          offset: { line: 1, column: 4 },
          map: {
            version: 3,
            sections: [
              {
                offset: { line: 0, column: 0 },
                map: {
                  version: 3,
                  sources: ['second.mdx'],
                  mappings: 'AAGE;AACA',
                },
              },
            ],
          },
        },
        { offset: { line: 3, column: 0 }, map: { version: 3, sections: [] } },
      ],
    }
    expect(
      remapMdxError(
        `${filename}:1:3: first\n${filename}:2:5: second\n${filename}:3:1: nested\n${filename}:4:1: gap`,
        filename,
        JSON.stringify(input),
      ).message,
    ).toBe(
      `first.mdx:1:1: first\nsecond.mdx:4:3: second\nsecond.mdx:5:3: nested\n${filename}:4:1 (in compiled MDX): gap`,
    )
  })

  it('accepts an empty indexed map and labels the unmapped compiled position', () => {
    expect(
      remapMdxError(`${filename}:1:1: missing`, filename, {
        version: 3,
        sections: [],
      }).message,
    ).toBe(`${filename}:1:1 (in compiled MDX): missing`)
  })

  it.each([
    { version: 3, sections: null },
    { version: 3, sections: [null] },
    { version: 3, sections: [{ offset: { line: -1, column: 0 } }] },
    { version: 3, sections: [{ offset: { line: '0', column: 0 } }] },
    { version: 3, sections: [{ offset: { line: 0, column: -1 } }] },
    { version: 3, sections: [{ offset: { line: 0, column: '0' } }] },
    { version: 3, sections: [{ offset: { line: 0, column: 0.5 } }] },
    {
      version: 3,
      sections: [0, 0].map(() => ({
        offset: { line: 0, column: 0 },
        map: { version: 3, sections: [] },
      })),
    },
    {
      version: 3,
      sections: [1, 0].map((line) => ({
        offset: { line, column: 0 },
        map: { version: 3, sections: [] },
      })),
    },
    {
      version: 3,
      sections: [
        {
          offset: { line: 0, column: Number.MAX_SAFE_INTEGER },
          map: { version: 3, sources: ['page.mdx'], mappings: 'CAAA' },
        },
      ],
    },
  ])(
    'diagnoses invalid indexed maps with the original filename: %j',
    (input) => {
      expect(() => remapMdxError(`${filename}:1:1`, filename, input)).toThrow(
        `${filename}:1:1: Invalid MDX source map:`,
      )
    },
  )
})
