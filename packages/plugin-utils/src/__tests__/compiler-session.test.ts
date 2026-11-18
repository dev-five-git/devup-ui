import * as wasm from '@devup-ui/wasm'
import { afterEach, describe, expect, it } from 'bun:test'

import { openCompilerSession } from '../compiler-session'
import type { DevupCompiler } from '../compiler-types'

const compiler: DevupCompiler = wasm
const pkg = '@devup-ui/react'
const source = `import { Box } from '${pkg}'; export const view = <Box color="red" />`

function extract(target: DevupCompiler, file: string) {
  const output = target.codeExtract(
    file,
    source,
    pkg,
    'df',
    true,
    false,
    false,
    {},
  )
  try {
    return output.code
  } finally {
    output.free()
  }
}

afterEach(() => {
  compiler.resetStateForTesting()
})

describe('openCompilerSession', () => {
  it('keeps each session apart on one shared compiler', () => {
    // Given: two apps, each with its own session and prefix.
    const first = openCompilerSession(compiler)
    const second = openCompilerSession(compiler)
    expect(first.id).not.toBe(second.id)
    first.compiler.setPrefix('one-')
    second.compiler.setPrefix('two-')

    // When: their calls interleave on the shared compiler.
    const one = extract(first.compiler, 'src/one.tsx')
    const two = extract(second.compiler, 'src/two.tsx')
    const again = extract(first.compiler, 'src/one.tsx')

    // Then: each call ran on its own session's state.
    expect(one).toContain('one-')
    expect(two).toContain('two-')
    expect(again).toBe(one)
    expect(first.compiler.getCss(null, false)).not.toContain('two-')
    expect(second.compiler.getCss(null, false)).not.toContain('one-')
    first.close()
    second.close()
  })

  it('reuses one wrapper per function and passes other values through', () => {
    // Given: a session over the module namespace.
    const session = openCompilerSession(compiler)

    // When / Then: function reads stay stable, other reads are untouched.
    expect(session.compiler.getCss).toBe(session.compiler.getCss)
    expect(Reflect.get(session.compiler, Symbol.toStringTag)).toBe('Module')
    session.close()
  })

  it('drops the session on close so later calls through it throw', () => {
    // Given: a closed session.
    const session = openCompilerSession(compiler)
    session.close()

    // When / Then: selecting it again is refused by the engine.
    expect(() => session.compiler.getCss(null, false)).toThrow(
      `engine session ${session.id} does not exist`,
    )
    expect(compiler.getCss(null, false)).toBeString()
  })
})
