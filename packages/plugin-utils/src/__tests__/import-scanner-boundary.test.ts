import { expect, it } from 'bun:test'

import { scanImports } from '../import-scanner'

it.each(
  ['function f() {}', 'class C {}'].flatMap((body) =>
    [
      'const x = 1\n',
      'async // marker\n',
      'const x = call()\n',
      'const x = []\n',
    ].map((prefix) => ({
      code: `${prefix}${body} /import("phantom")/.test(x); import("./real");`,
    })),
  ),
)('masks regex after an ASI declaration in $code', ({ code }) => {
  // Given: valid syntax, checked without evaluating the fixture.
  new Bun.Transpiler({ loader: 'js' }).transformSync(code)
  // When: scan through the real lexical boundary.
  const actual = scanImports(code, false)
  // Then: regex text adds no phantom dependency.
  expect(actual).toEqual([{ kind: 'dynamic', specifier: './real' }])
})

it.each(
  ['function f() {}', 'class C {}'].flatMap((body) =>
    [
      ['const value =\n', ';'],
      ['const value = (\n', ');'],
      ['const value = [\n', '];'],
      ['const value = { field:\n', '};'],
      ['const value = ok ?\n', ': 0;'],
      ['call(0,\n', ');'],
      ['function outer() { return ', '; }'],
      ['function outer() { throw ', '; }'],
      ['const value = void\n', ';'],
      ['const value = typeof\n', ';'],
      ['const value = delete\n', ';'],
      ['const value = new\n', ';'],
      ['const value = !\n', ';'],
      ['const value = ~\n', ';'],
      ['const value = +\n', ';'],
      ['const value = -\n', ';'],
    ].map(([prefix, suffix]) => ({
      code: `${prefix}${body} / import("./real")${suffix}`,
    })),
  ),
)('keeps literal imports in expression division in $code', ({ code }) => {
  // Given: each expression prefix is real syntax, not a token-only example.
  new Bun.Transpiler({ loader: 'js' }).transformSync(code)
  // When: scan without executing the expression or resolving its import.
  const actual = scanImports(code, false)
  // Then: division retains the literal import dependency.
  expect(actual).toEqual([{ kind: 'dynamic', specifier: './real' }])
})
