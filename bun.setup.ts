import './bun.matrix'

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
