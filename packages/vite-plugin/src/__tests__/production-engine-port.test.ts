import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'

import { operation } from '../../../plugin-utils/src/__tests__/generation-fixture'
import { productionEngine, ProductionLifecycle } from '../production-engine'

it('replaces every generation-local flag after the binding faults outside an action', () => {
  // Given
  const lifecycle = new ProductionLifecycle()
  const previous = lifecycle.generation
  const cause = new TypeError('checkpoint failure')
  const engine = {
    ...productionEngine,
    capture() {
      throw cause
    },
  }
  previous.owner.run(
    engine,
    operation(() => {
      wasm.registerTheme({})
      wasm.registerShorthands({})
      wasm.setDebug(false)
      const output = wasm.codeExtract(
        'partial.tsx',
        "import {Box} from '@devup-ui/react'; export const x=<Box color='red'/>;",
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
      )
      output.free()
    }),
  )
  expect(() => wasm.setPrefix('unmanaged')).toThrow('CaptureFailed')
  // When
  try {
    const replaced = lifecycle.input()
    // Then
    expect(replaced).toBe(true)
    expect(lifecycle.generation).not.toBe(previous)
    expect(previous.owner.disposed).toBe(true)
    expect(lifecycle.generation.contexts.size).toBe(0)
    expect(lifecycle.generation.numbering).toBeUndefined()
    expect(lifecycle.generation.manifest).toBeUndefined()
    expect(lifecycle.generation.pending).toBeUndefined()
    expect(lifecycle.generation.current).toBeUndefined()
  } finally {
    lifecycle.generation.dispose()
  }
})
