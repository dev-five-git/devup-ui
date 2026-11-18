import { readFileSync } from 'node:fs'

import { expect, test } from '@playwright/test'

// The screenshot baselines show the benchmark section, so the e2e builds render
// the frozen e2e/fixtures/benchmark-results.json instead of the live
// benchmark-results.json that CI rewrites after every merge to main. These
// checks fail when a build under test reads the live file, which would make a
// benchmark refresh change the screenshots.
const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/benchmark-results.json', import.meta.url),
    'utf8',
  ),
) as {
  run: { id: string }
  rows: { table: string; target: string; median: number; bytes: number }[]
}

test.describe('Landing Page - Benchmark fixture', () => {
  test('the benchmark section is built from the e2e fixture', async ({
    page,
  }) => {
    await page.goto('/')
    const section = page.locator('[data-benchmark-source]')
    await expect(section).toHaveCount(1)
    await expect(section).toHaveAttribute('data-benchmark-source', 'fixture')
  })

  test('the benchmark section shows the fixture numbers', async ({ page }) => {
    const devup = fixture.rows.find(
      (row) => row.table === 'webpack' && row.target === 'devup-ui-single',
    )
    if (!devup)
      throw new Error('the fixture has no webpack devup-ui-single row')

    await page.goto('/')
    const section = page.locator('[data-benchmark-source]')
    await expect(section).toContainText(`run ${fixture.run.id}`)
    await expect(section).toContainText(`${devup.median.toFixed(2)}s`)
    await expect(section).toContainText(
      `${(devup.bytes / 1024 / 1024).toFixed(2)}MB`,
    )
  })
})
