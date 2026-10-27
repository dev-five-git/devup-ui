import { BuildGeneration } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { productionEngine } from '../production-engine'
import { extractOwned, productionFixture } from './production-fixture'

function capturedError(action: () => void): Error {
  try {
    action()
  } catch (error) {
    if (!(error instanceof Error)) throw error
    return error
  }
  throw new TypeError('Expected the closed owner to throw')
}

it.each([false, true])(
  'disposes activation when capture fails and starts fresh at next input (action fault=%s)',
  async (actionFault) => {
    // Given an actual prepared activation and its successful private handoff.
    const fixture = await productionFixture()
    await fixture.file('app/src/main.js', 'export const value=1;')
    const activation = await fixture.activation()
    const cause = new TypeError('capture fault')
    try {
      await activation.prepare(() => {})
      const key = activation.context
      const modules = activation.modules(key)
      const closed = new BuildGeneration<
        ReturnType<typeof productionEngine.capture>
      >()
      closed.dispose()
      const expectedClosed = capturedError(() =>
        closed.run(productionEngine, () => undefined),
      )
      expect(expectedClosed.name).toBe('ClosedBuildGenerationError')
      activation.run(key, () => extractOwned('red'))
      const capture = spyOn(productionEngine, 'capture').mockImplementation(
        () => {
          throw cause
        },
      )
      try {
        // When real action allocations are followed by a broken final capture.
        expect(() =>
          activation.run(key, () => {
            const output = wasm.codeExtract(
              'fault.tsx',
              "import {Box} from '@devup-ui/react'; export const x=<Box bg='blue'/>;",
              '@devup-ui/react',
              'df',
              false,
              false,
              false,
              {},
            )
            output.free()
            if (actionFault) throw new RangeError('action fault')
          }),
        ).toThrow(cause)
      } finally {
        capture.mockRestore()
      }
      // Then the integration is terminal, and a new input creates a fresh owner.
      const actualClosed = capturedError(() => activation.run(key, () => 1))
      expect(actualClosed.constructor).toBe(expectedClosed.constructor)
      expect(actualClosed.name).toBe(expectedClosed.name)
      await activation.prepare(
        () => {},
        () => {},
      )
      expect(activation.modules(activation.context)).not.toBe(modules)
      const fresh = activation.run(activation.context, () =>
        extractOwned('green'),
      )
      expect(fresh.css).toContain('background:green')
      expect(fresh.css).not.toContain('background:red')
      expect(fresh.css).not.toContain('background:blue')
    } finally {
      activation.dispose()
      await fixture.close()
    }
  },
)
