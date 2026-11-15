import { createRequire } from 'node:module'

export const wasm = createRequire(import.meta.url)('./pkg/index.js')
export const api = wasm.ownership
export const ownerSite = {
  file: '/src/owner.tsx',
  line: 3,
  column: 5,
  operation: 'extract',
}
export const snapshot = () => ({
  sheet: wasm.exportSheet(),
  classes: wasm.exportClassMap(),
  files: wasm.exportFileMap(),
  canonical: wasm.exportCanonicalMap(),
  reservation: wasm.exportReservationState(),
})
export const caught = (action) => {
  try {
    action()
  } catch (error) {
    return { error }
  }
  throw new Error('expected failure')
}
export const close = (owner) => {
  api.dispose(owner)
  api.clear(owner)
}
