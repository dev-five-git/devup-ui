export const wasmBindgenVersionPattern =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?![\s\S])/

export function resolveWasmBindgenVersion(lockText) {
  const lock = globalThis.Bun.TOML.parse(lockText)
  if (!Array.isArray(lock.package))
    throw new Error('resolve-wasm-bindgen: missing package array')
  const versions = new Set()
  for (const entry of lock.package) {
    if (entry?.name !== 'wasm-bindgen') continue
    const version = entry.version
    if (typeof version !== 'string' || !wasmBindgenVersionPattern.test(version))
      throw new Error('resolve-wasm-bindgen: invalid wasm-bindgen version')
    versions.add(version)
  }
  if (versions.size !== 1)
    throw new Error(
      'resolve-wasm-bindgen: expected one unique wasm-bindgen version',
    )
  return [...versions][0]
}
