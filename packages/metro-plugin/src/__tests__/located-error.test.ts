import { describe, expect, it } from 'bun:test'

import { codeFrame, DevupUIBuildError, locatedError } from '../located-error'

const source = ['a', 'bb', '', 'dddd', 'e', 'f'].join('\n')

describe('codeFrame', () => {
  it('shows two lines around the line with a caret under the column', () => {
    expect(codeFrame(source, 4, 3)).toBe(
      [
        '  2 | bb',
        '  3 |',
        '> 4 | dddd',
        '    |   ^',
        '  5 | e',
        '  6 | f',
      ].join('\n'),
    )
  })

  it('stops at the first and the last line', () => {
    expect(codeFrame('x\r\ny', 1, 1)).toBe(
      ['> 1 | x', '    | ^', '  2 | y'].join('\n'),
    )
    expect(codeFrame('x\ny', 2, 1)).toBe(
      ['  1 | x', '> 2 | y', '    | ^'].join('\n'),
    )
  })

  it('pads the gutter to the widest line number', () => {
    const lines = Array.from({ length: 10 }, (_, index) => `l${index + 1}`)
    expect(codeFrame(lines.join('\n'), 9, 2)).toBe(
      ['   7 | l7', '   8 | l8', '>  9 | l9', '     |  ^', '  10 | l10'].join(
        '\n',
      ),
    )
  })
})

describe('locatedError', () => {
  it('locates an engine error at its first location', () => {
    const cause = new Error(
      'src/App.tsx:4:3: `<Box>` cannot use `_hover` on React Native\nsrc/App.tsx:5:1: another',
    )
    const error = locatedError(cause, 'src/App.tsx', source)
    expect(error).toBeInstanceOf(DevupUIBuildError)
    expect(error).toBeInstanceOf(SyntaxError)
    if (!(error instanceof DevupUIBuildError)) throw error
    expect(error.name).toBe('DevupUIBuildError')
    expect(error.message).toBe(cause.message)
    expect(error.loc).toEqual({ line: 4, column: 2 })
    expect(error.codeFrame).toBe(codeFrame(source, 4, 3))
    expect(error.cause).toBe(cause)
  })

  it('matches the filename literally', () => {
    const cause = new Error('a+b(1).tsx:1:1: bad')
    expect(locatedError(cause, 'a+b(1).tsx', 'x')).toBeInstanceOf(
      DevupUIBuildError,
    )
    expect(locatedError(cause, 'aab1.tsx', 'x')).toBe(cause)
  })

  it('keeps an error without a location as it is', () => {
    const cause = new Error('unknown React Native platform ``')
    expect(locatedError(cause, 'App.tsx', 'x')).toBe(cause)
    expect(locatedError('text', 'App.tsx', 'x')).toBe('text')
  })
})
