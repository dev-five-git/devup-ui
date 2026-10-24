// node verify-wasm-size.js --base-pkg DIR --candidate-pkg DIR --metadata FILE
//   --report FILE [--summary FILE] [--config FILE]
// Without --summary, GITHUB_STEP_SUMMARY is required. No baseline writer exists.
import { cli } from './wasm-size.js'

try {
  process.exitCode = cli(process.argv.slice(2))
} catch (error) {
  if (!(error instanceof Error)) throw error
  console.error('verify-wasm-size: failed', error.message)
  process.exitCode = 1
}
