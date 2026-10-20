// bun resolve-wasm-bindgen.js /absolute/path/to/Cargo.lock
import { readFileSync } from 'node:fs'
import { isAbsolute } from 'node:path'

import { resolveWasmBindgenVersion } from './wasm-bindgen-version.js'

try {
  const args = process.argv.slice(2)
  if (args.length !== 1 || !isAbsolute(args[0]))
    throw new Error('expected exactly one absolute Cargo.lock path')
  console.info(resolveWasmBindgenVersion(readFileSync(args[0], 'utf8')))
} catch (error) {
  if (!(error instanceof Error)) throw error
  console.error('resolve-wasm-bindgen: failed', error.message)
  process.exitCode = 1
}
