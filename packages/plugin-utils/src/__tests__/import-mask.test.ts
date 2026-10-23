import { expect, it } from 'bun:test'

import { maskImportText } from '../import-mask'

it('masks escaped strings, nested templates and comments without consuming subsequent code', () => {
  const code = [
    String.raw`const quote = "escaped \" import './fake'";`,
    'const value = `outer ${ {a: "}", b: `nested ${1}`} /* comment */ // comment',
    '} import fake`;',
    '/* multiline\n import fake */',
    '// import fake',
    "import './real'",
  ].join('\n')
  const masked = maskImportText(code, false)
  expect(masked).not.toContain('fake')
  expect(masked).toContain('import')
  expect(masked.length).toBe(code.length)
})

it('masks regex quotes, escaped slashes and character classes', () => {
  const code = String.raw`const value = /[/'"]import '\/fake'/gi; return /import "fake"/; import './real'`
  expect(maskImportText(code, false)).not.toContain('fake')
})

it('masks JSX fragments, nesting, attributes and self-closing tags but keeps JavaScript expressions', () => {
  const code =
    'const value = <><div title="import fake">import fake { {value: import("real")} }<span/>tail</div></>; import "after"'
  const masked = maskImportText(code, true)
  expect(masked).not.toContain('fake')
  expect(masked.match(/\bimport\b/g)).toHaveLength(2)
})

it('keeps TypeScript assertions and division as code rather than JSX or regex', () => {
  const code =
    'const result = <number>value / 2; const object = {x: 1}; import "real"'
  expect(maskImportText(code, false)).toContain('<number>value / 2')
})

it('handles JSX attribute expressions', () => {
  const masked = maskImportText(
    'const node = <div title={import("real")}>import fake</div>;',
    true,
  )
  expect(masked).toContain('import')
  expect(masked).not.toContain('fake')
})

it('masks regex literals following control parentheses', () => {
  expect(
    maskImportText(
      'if (true) /import "fake"/.test(value); import "real"',
      false,
    ),
  ).not.toContain('fake')
})
