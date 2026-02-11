import { expect, test } from '@playwright/test'

/**
 * Mock the GitHub API to return a fixed star count so screenshots are deterministic.
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

async function enableDarkMode(page: import('@playwright/test').Page) {
  await page.emulateMedia({ colorScheme: 'dark' })
}

async function setDarkThemeAttribute(page: import('@playwright/test').Page) {
  await page.evaluate(() =>
    document.documentElement.setAttribute('data-theme', 'dark'),
  )
  await page.waitForTimeout(300)
}

test.describe('Landing Page - Dark Mode Visual Regression', () => {
  test.describe('Full page screenshots (dark)', () => {
    test('full page dark at mobile (375px)', async ({ page }) => {
      await mockGitHubStars(page)
      await enableDarkMode(page)
      await page.setViewportSize({ width: 375, height: 812 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await setDarkThemeAttribute(page)
      await page.waitForTimeout(1000)

      await expect(page).toHaveScreenshot('dark-full-page-mobile.png', {
        fullPage: true,
        maxDiffPixelRatio: 0.02,
      })
    })

    test('full page dark at desktop (1440px)', async ({ page }) => {
      await mockGitHubStars(page)
      await enableDarkMode(page)
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await setDarkThemeAttribute(page)
      await page.waitForTimeout(1000)

      await expect(page).toHaveScreenshot('dark-full-page-desktop.png', {
        fullPage: true,
        maxDiffPixelRatio: 0.02,
      })
    })
  })

  test.describe('Section screenshots (dark)', () => {
    test.beforeEach(async ({ page }) => {
      await mockGitHubStars(page)
      await enableDarkMode(page)
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await setDarkThemeAttribute(page)
      await page.waitForTimeout(1000)
    })

    test('TopBanner section (dark)', async ({ page }) => {
      const topBanner = page
        .locator('div')
        .filter({
          hasText: /Zero Config/,
        })
        .first()

      await expect(topBanner).toBeVisible()
      await expect(topBanner).toHaveScreenshot('dark-section-top-banner.png', {
        maxDiffPixelRatio: 0.02,
      })
    })

    test('Feature section (dark)', async ({ page }) => {
      const featureHeading = page.getByText('Features', { exact: true }).first()
      await featureHeading.scrollIntoViewIfNeeded()
      await page.waitForTimeout(300)

      const featureSection = featureHeading
        .locator('..')
        .locator('..')
        .locator('..')

      await expect(featureSection).toHaveScreenshot(
        'dark-section-features.png',
        {
          maxDiffPixelRatio: 0.02,
        },
      )
    })

    test('Bench section (dark)', async ({ page }) => {
      const benchHeading = page.getByText('Comparison Bechmarks').first()
      await benchHeading.scrollIntoViewIfNeeded()
      await page.waitForTimeout(300)

      const benchSection = benchHeading
        .locator('..')
        .locator('..')
        .locator('..')

      await expect(benchSection).toHaveScreenshot('dark-section-bench.png', {
        maxDiffPixelRatio: 0.02,
      })
    })

    test('Discord section (dark)', async ({ page }) => {
      const discordHeading = page.getByText('Join our community').first()
      await discordHeading.scrollIntoViewIfNeeded()
      await page.waitForTimeout(300)

      const discordSection = discordHeading
        .locator('..')
        .locator('..')
        .locator('..')

      await expect(discordSection).toHaveScreenshot(
        'dark-section-discord.png',
        {
          maxDiffPixelRatio: 0.02,
        },
      )
    })

    test('Footer section (dark)', async ({ page }) => {
      const footer = page.locator('footer')
      await footer.scrollIntoViewIfNeeded()
      await page.waitForTimeout(300)

      await expect(footer).toHaveScreenshot('dark-section-footer.png', {
        maxDiffPixelRatio: 0.02,
      })
    })
  })
})
