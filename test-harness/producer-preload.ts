import { readFileSync } from 'node:fs'
import { Session } from 'node:inspector/promises'

import { afterAll } from 'bun:test'

import { PRODUCER_TIMEOUT_MS } from './producer-timeout'

const session = new Session()
session.connect()
await session.post('Profiler.enable')
await session.post('Profiler.startPreciseCoverage', {
  callCount: true,
  detailed: true,
})

async function capture(phase: string): Promise<void> {
  const continuation = new Promise((resolve) =>
    process.stdin.once('data', resolve),
  )
  const profile = await session.post('Profiler.takePreciseCoverage')
  process.stdout.write(
    `${process.env['DEVUP_PRODUCER_TOKEN']} ${JSON.stringify({
      phase,
      pid: process.pid,
      bun: Bun.version,
      revision: Bun.revision,
      config: readFileSync(process.env['DEVUP_PRODUCER_CONFIG'] ?? '', 'utf8'),
      profiles: profile.result,
    })}\n`,
  )
  await continuation
}

await capture('bootstrap')
afterAll(() => capture('final'), PRODUCER_TIMEOUT_MS)
