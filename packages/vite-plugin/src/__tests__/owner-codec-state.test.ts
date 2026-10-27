import { BuildGeneration } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'

import { productionEngine, type ProductionState } from '../production-engine'

it.each([false, true])(
  'retains all five owner fields after action failure and sibling work (singleCss=%s)',
  (singleCss) => {
    // Given two independent real owners with live reservation bookkeeping.
    const a = new BuildGeneration<ProductionState>()
    const b = new BuildGeneration<ProductionState>()
    const configure = () => {
      wasm.registerTheme({})
      wasm.registerShorthands({})
      wasm.setDebug(false)
      wasm.setPrefix(null)
      wasm.setModuleResolver(undefined)
    }
    const extract = (id: string, color: string) => {
      const output = wasm.codeExtract(
        id,
        `import {Box} from '@devup-ui/react'; export const x=<Box bg='${color}'/>;`,
        '@devup-ui/react',
        'df',
        singleCss,
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
    let retained: ProductionState | undefined
    try {
      expect(() =>
        a.run(productionEngine, () => {
          configure()
          wasm.reserveClassSlots(
            [
              [
                'bad.tsx',
                'import {Box} from "@devup-ui/react"; const x=<Box',
                'df',
              ],
            ],
            '@devup-ui/react',
            singleCss,
            false,
            false,
            {},
          )
          extract('01\0𐀀.tsx', 'red')
          retained = productionEngine.capture()
          throw new RangeError('action fault')
        }),
      ).toThrow(RangeError)
      b.run(productionEngine, () => {
        configure()
        extract('sibling.tsx', 'blue')
      })
      // When A resumes for a CSS-only read after B overwrote every global.
      const resumed = a.run(productionEngine, () => {
        configure()
        const state = productionEngine.capture()
        const files: Record<string, number> = JSON.parse(state.files)
        return {
          state,
          css:
            wasm.getCss(null, false) +
            Object.values(files)
              .map((number) => wasm.getCss(number, false))
              .join(''),
          misses: wasm.getUnreservedClassSlotCount(),
          suppressed: wasm.getSuppressedReservationRowCount(),
        }
      })
      // Then all five raw strings and reservation accounting survive the fault.
      expect(retained).toBeDefined()
      expect(retained).toEqual(resumed.state)
      expect(resumed.css).toContain('background:red')
      expect(resumed.misses).toBeGreaterThan(0)
      expect(resumed.suppressed).toBe(1)
      const next = a.run(productionEngine, () => {
        configure()
        extract('next.tsx', 'green')
        return wasm.getUnreservedClassSlotCount()
      })
      expect(next).toBeGreaterThan(resumed.misses)
    } finally {
      a.dispose()
      b.dispose()
      wasm.resetBuildState()
    }
  },
)
