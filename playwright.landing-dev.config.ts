import { defineConfig, devices } from '@playwright/test'

const next = process.env.LANDING_BUILD_MODE === 'next'

export default defineConfig({
  testDir: './e2e',
  testMatch: '**/landing-dev.spec.ts',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: 'list',
  timeout: 90_000,
  use: {
    baseURL: 'http://localhost:3098',
    viewport: { width: 1440, height: 900 },
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: next
      ? 'node ./script.js && bun x next dev --port 3098'
      : 'bun run dev --port 3098 --strictPort',
    cwd: './apps/landing',
    url: 'http://localhost:3098',
    reuseExistingServer: false,
    timeout: 120_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
})
