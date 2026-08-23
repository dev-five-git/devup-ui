import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  snapshotPathTemplate:
    '{snapshotDir}/{testFileDir}/{testFileName}-snapshots/{arg}-{platform}{ext}',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : 4,
  reporter: 'html',
  timeout: 60_000,
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.01,
    },
  },
  use: {
    baseURL: 'http://localhost:3099',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        launchOptions: {
          args: [
            '--font-render-hinting=none',
            '--disable-font-subpixel-positioning',
            '--disable-skia-runtime-opts',
            '--disable-lcd-text',
          ],
        },
      },
    },
  ],
  webServer: {
    // LANDING_BUILD_MODE=next runs the same suite against the CI-only Next
    // build, which keeps @devup-ui/next-plugin's Turbopack production path
    // gated by these assertions and pixels.
    command: process.env.CI
      ? 'node e2e/serve-static.mjs 3099'
      : `bun run --filter landing ${process.env.LANDING_BUILD_MODE === 'next' ? 'build:next' : 'build'} && node e2e/serve-static.mjs 3099`,
    url: 'http://localhost:3099',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    stdout: 'pipe',
  },
})
