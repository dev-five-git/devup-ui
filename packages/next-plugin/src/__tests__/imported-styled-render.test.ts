import { expect, it } from 'bun:test'

import { assertImportedStyledRendering } from './imported-styled-render'

it('rejects unresolved imports rather than fabricating a module binding', async () => {
  // Given: a complete emission requesting a module outside this exact fixture graph.
  const codes = { app: "import 'unresolved';", base: '', card: '' }
  // When/Then: the full-module linker refuses it before reporting render semantics.
  await expect(assertImportedStyledRendering(codes, '')).rejects.toThrow(
    new SyntaxError('Unexpected emitted import unresolved'),
  )
})
