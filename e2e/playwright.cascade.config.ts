import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: '.',
  testMatch: 'environment-cascade-pair.spec.ts',
  outputDir: join(tmpdir(), `devup-cascade-results-${process.pid}`),
  workers: 1,
  retries: 0,
  timeout: 60_000,
  globalTimeout: 600_000,
  reporter: 'line',
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
})
