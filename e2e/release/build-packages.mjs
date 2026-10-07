import { writeFile } from 'node:fs/promises'

import { runGuarded } from './guarded-process.mjs'

const measurements = []
const findings = []
try {
  await runGuarded(process.env.BUN_BINARY ?? 'bun', ['run', 'build'], {
    timeoutMs: 1_200_000,
    onResult: (result) => measurements.push(result),
  })
} catch (error) {
  if (!(error instanceof Error)) throw error
  findings.push({ phase: 'package-build', message: error.message })
  throw error
} finally {
  await writeFile(
    'release-package-build.json',
    JSON.stringify({ findings, measurements }, null, 2),
  )
}
