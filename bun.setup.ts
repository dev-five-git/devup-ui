import { afterEach, beforeEach, spyOn } from 'bun:test'

import { resetWasmForTesting } from './packages/next-plugin/src/wasm'
import { createModuleResolver } from './packages/plugin-utils/src'

const { resetStateForTesting, setDebug, setModuleResolver } =
  await import('./bindings/devup-ui-wasm/pkg/index.js')
const testModuleResolver = createModuleResolver()
const nativeBunPlugin = Bun.plugin
export const bunRegistration = Object.assign(spyOn(Bun, 'plugin'), {
  clearAll: nativeBunPlugin.clearAll,
})
try {
  await import('./packages/bun-plugin/src/plugin')
} finally {
  bunRegistration.mockRestore()
  // Restoring the global property leaves the retained import callable inert.
  bunRegistration.mockImplementation(nativeBunPlugin)
}

function resetTestState() {
  resetStateForTesting()
  setDebug(true)
  setModuleResolver(testModuleResolver)
  resetWasmForTesting()
}

beforeEach(resetTestState)
afterEach(resetTestState)

function cartesianProduct<T extends any[][]>(arrays: T) {
  return arrays.reduce(
    (acc, curr) => acc.flatMap((x) => curr.map((y) => [...x, y])),
    [[]],
  )
}

function createTestMatrix<T extends Record<string, any[]>>(
  optionsMap: T,
): {
  [K in keyof T]: T[K] extends (infer U)[] ? U : never
}[] {
  const keys = Object.keys(optionsMap)
  const values = Object.values(optionsMap)

  return cartesianProduct(values).map<{
    [K in keyof T]: T[K] extends (infer U)[] ? U : never
  }>(
    (combination) =>
      Object.fromEntries(keys.map((key, i) => [key, combination[i]])) as {
        [K in keyof T]: T[K] extends (infer U)[] ? U : never
      },
  )
}

globalThis.createTestMatrix = createTestMatrix

declare global {
  function createTestMatrix<T extends Record<string, any[]>>(
    optionsMap: T,
  ): {
    [K in keyof T]: T[K] extends (infer U)[] ? U : never
  }[]
}
