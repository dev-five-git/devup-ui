import { defineConfig, devices } from '@playwright/test'

/**
 * Gates the static output of apps/vinext-rsc-css-repro (vinext + RSC). Build
 * it first: `bun run --filter vinext-rsc-css-repro build`, or with
 * DEVUP_SINGLE_CSS=1 for the singleCss variant.
 */
export default defineConfig({
  testDir: './apps/vinext-rsc-css-repro/e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : 4,
  reporter: 'list',
  timeout: 60_000,
  use: {
    baseURL: 'http://localhost:3187',
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'node e2e/serve-static.mjs 3187',
    env: {
      LANDING_OUTPUT_ROOT: 'apps/vinext-rsc-css-repro/dist/client',
    },
    url: 'http://localhost:3187',
    reuseExistingServer: false,
    timeout: 60_000,
    stdout: 'pipe',
  },
})
