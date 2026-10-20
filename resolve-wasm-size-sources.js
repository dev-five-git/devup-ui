import { appendFileSync } from 'node:fs'

import { cli } from './wasm-size-sources.js'

try {
  process.exitCode = cli(process.env)
} catch (error) {
  if (!(error instanceof Error)) throw error
  console.error('resolve-wasm-size-sources: failed', error.message)
  if (process.env.GITHUB_STEP_SUMMARY)
    appendFileSync(
      process.env.GITHUB_STEP_SUMMARY,
      `## WASM size: FAIL - source resolution\n\n\`\`\`text\n${error.message}\n\`\`\`\n\n`,
    )
  process.exitCode = 1
}
