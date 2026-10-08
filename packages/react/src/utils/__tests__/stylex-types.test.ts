import { describe, expect, it } from 'bun:test'

import { compileStylex } from '../../../type-tests/compile-stylex'

describe('StyleX transition consumers', () => {
  it('accepts valid consumers with the negative fixtures compiler options', async () => {
    // Given a consumer of every advertised import form and accepted value kind.
    const fixture = './stylex-transitions.ts'
    // When the installed native compiler checks the consumer.
    const result = await compileStylex(fixture)
    // Then no diagnostic or setup failure is accepted.
    expect(result).toEqual({ exitCode: 0, diagnostics: '' })
  })

  it.each([
    ['flat-view', 'TS2353'],
    ['pseudo-slot', 'TS2353'],
    ['root-slot', 'TS2353'],
    ['scalar-slot', 'TS2322'],
    ['null-slot', 'TS2322'],
    ['false-slot', 'TS2322'],
    ['undefined-slot', 'TS2379'],
    ['array-slot', 'TS2322'],
    ['function-slot', 'TS2322'],
    ['condition-leaf', 'TS2322'],
    ['array-leaf', 'TS2322'],
    ['function-leaf', 'TS2322'],
    ['true-leaf', 'TS2322'],
    ['position-key', 'TS2353'],
    ['position-condition', 'TS2322'],
    ['position-array', 'TS2322'],
    ['position-function', 'TS2322'],
    ['position-true', 'TS2322'],
    ['subpath-default', 'TS1192'],
  ])('rejects %s with a fixture-located diagnostic', async (name, code) => {
    // Given one invalid consumer without a type suppression.
    const filename = `${name}.ts`
    // When the same compiler checks that consumer.
    const result = await compileStylex(`./stylex-invalid/${filename}`)
    // Then a compiler failure must identify the fixture and diagnostic code.
    expect(result.exitCode).not.toBe(0)
    expect(result.diagnostics).toMatch(
      new RegExp(
        `${filename.replaceAll('.', '\\.')}\\(\\d+,\\d+\\): error ${code}:`,
      ),
    )
  })
})
