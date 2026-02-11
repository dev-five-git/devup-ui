import { expect, test } from '@playwright/test'

/**
 * Mock the GitHub API to return a fixed star count so screenshots are deterministic.
 * StarButton fetches: https://api.github.com/repos/dev-five-git/devup-ui
 */
async function mockGitHubStars(page: import('@playwright/test').Page) {
  await page.route('**/api.github.com/repos/dev-five-git/devup-ui', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        stargazers_count: 1234,
        full_name: 'dev-five-git/devup-ui',
      }),
    }),
  )
}

test.describe('Landing Page - Visual Regression', () => {
  test.describe('Full page screenshots', () => {
    test('full page at mobile (375px)', async ({ page }) => {
      await mockGitHubStars(page)
      await page.setViewportSize({ width: 375, height: 812 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      // Wait for fonts and images
      await page.waitForTimeout(1000)

      await expect(page).toHaveScreenshot('full-page-mobile.png', {
        fullPage: true,
        maxDiffPixelRatio: 0.02,
      })
    })

    test('full page at desktop (1440px)', async ({ page }) => {
      await mockGitHubStars(page)
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await page.waitForTimeout(1000)

      await expect(page).toHaveScreenshot('full-page-desktop.png', {
        fullPage: true,
        maxDiffPixelRatio: 0.02,
      })
    })
  })

  test.describe('Section screenshots at desktop', () => {
    test.beforeEach(async ({ page }) => {
      await mockGitHubStars(page)
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await page.waitForTimeout(1000)
    })

    test('TopBanner section', async ({ page }) => {
      // TopBanner is the first section containing "Zero Config"
      const topBanner = page
        .locator('div')
        .filter({
          hasText: /Zero Config/,
        })
        .first()

      await expect(topBanner).toBeVisible()
      await expect(topBanner).toHaveScreenshot('section-top-banner.png', {
        maxDiffPixelRatio: 0.02,
      })
    })

    test('Feature section', async ({ page }) => {
      const featureHeading = page.getByText('Features', { exact: true }).first()
      await featureHeading.scrollIntoViewIfNeeded()
      await page.waitForTimeout(300)

      // Get the feature section container
      const featureSection = featureHeading
        .locator('..')
        .locator('..')
        .locator('..')

      await expect(featureSection).toHaveScreenshot('section-features.png', {
        maxDiffPixelRatio: 0.02,
      })
    })

    test('Bench section', async ({ page }) => {
      const benchHeading = page.getByText('Comparison Bechmarks').first()
      await benchHeading.scrollIntoViewIfNeeded()
      await page.waitForTimeout(300)

      const benchSection = benchHeading
        .locator('..')
        .locator('..')
        .locator('..')

      await expect(benchSection).toHaveScreenshot('section-bench.png', {
        maxDiffPixelRatio: 0.02,
      })
    })

    test('Discord section', async ({ page }) => {
      const discordHeading = page.getByText('Join our community').first()
      await discordHeading.scrollIntoViewIfNeeded()
      await page.waitForTimeout(300)

      const discordSection = discordHeading
        .locator('..')
        .locator('..')
        .locator('..')

      await expect(discordSection).toHaveScreenshot('section-discord.png', {
        maxDiffPixelRatio: 0.02,
      })
    })

    test('Footer section', async ({ page }) => {
      const footer = page.locator('footer')
      await footer.scrollIntoViewIfNeeded()
      await page.waitForTimeout(300)

      await expect(footer).toHaveScreenshot('section-footer.png', {
        maxDiffPixelRatio: 0.02,
      })
    })
  })
})
