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

test.describe('Landing Page - Hover State Visual Regression', () => {
  test.beforeEach(async ({ page }) => {
    await mockGitHubStars(page)
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(1000)
  })

  test('GetStarted button hover screenshot', async ({ page }) => {
    const getStartedLink = page.locator('a', { hasText: 'Get started' }).first()
    await expect(getStartedLink).toBeVisible()

    await expect(getStartedLink).toHaveScreenshot(
      'hover-get-started-before.png',
      { maxDiffPixelRatio: 0.02 },
    )

    await getStartedLink.hover()
    await page.waitForTimeout(300)

    await expect(getStartedLink).toHaveScreenshot(
      'hover-get-started-after.png',
      { maxDiffPixelRatio: 0.02 },
    )
  })

  test('Star button hover screenshot', async ({ page }) => {
    const starLink = page.locator('a', { hasText: /Star/i }).first()
    await expect(starLink).toBeVisible()

    await expect(starLink).toHaveScreenshot('hover-star-before.png', {
      maxDiffPixelRatio: 0.02,
    })

    await starLink.hover()
    await page.waitForTimeout(300)

    await expect(starLink).toHaveScreenshot('hover-star-after.png', {
      maxDiffPixelRatio: 0.02,
    })
  })

  test('Sponsor button hover screenshot', async ({ page }) => {
    const sponsorLink = page.locator('a', { hasText: /Sponsor/i }).first()
    await expect(sponsorLink).toBeVisible()

    await expect(sponsorLink).toHaveScreenshot('hover-sponsor-before.png', {
      maxDiffPixelRatio: 0.02,
    })

    await sponsorLink.hover()
    await page.waitForTimeout(300)

    await expect(sponsorLink).toHaveScreenshot('hover-sponsor-after.png', {
      maxDiffPixelRatio: 0.02,
    })
  })

  test('Discord button hover screenshot', async ({ page }) => {
    const discordLink = page.getByRole('link', { name: /Join our Discord/i })
    await discordLink.scrollIntoViewIfNeeded()
    await expect(discordLink).toBeVisible()

    await expect(discordLink).toHaveScreenshot('hover-discord-before.png', {
      maxDiffPixelRatio: 0.02,
    })

    await discordLink.hover()
    await page.waitForTimeout(300)

    await expect(discordLink).toHaveScreenshot('hover-discord-after.png', {
      maxDiffPixelRatio: 0.02,
    })
  })

  test('KakaoTalk button hover screenshot', async ({ page }) => {
    const kakaoLink = page.getByRole('link', { name: /Open KakaoTalk/i })
    await kakaoLink.scrollIntoViewIfNeeded()
    await expect(kakaoLink).toBeVisible()

    await expect(kakaoLink).toHaveScreenshot('hover-kakao-before.png', {
      maxDiffPixelRatio: 0.02,
    })

    await kakaoLink.hover()
    await page.waitForTimeout(300)

    await expect(kakaoLink).toHaveScreenshot('hover-kakao-after.png', {
      maxDiffPixelRatio: 0.02,
    })
  })

  test('Feature card hover screenshot', async ({ page }) => {
    const featureHeading = page.getByText('Features', { exact: true }).first()
    await featureHeading.scrollIntoViewIfNeeded()
    await page.waitForTimeout(300)

    // Find the first feature card in the grid
    const featureSection = featureHeading
      .locator('..')
      .locator('..')
      .locator('..')

    // Get the first card-like element in the feature section
    const firstCard = featureSection
      .locator('div[class]')
      .filter({
        hasText: /Zero Runtime|Zero Config|Smallest/,
      })
      .first()

    await expect(firstCard).toHaveScreenshot('hover-feature-card-before.png', {
      maxDiffPixelRatio: 0.02,
    })

    await firstCard.hover()
    await page.waitForTimeout(300)

    await expect(firstCard).toHaveScreenshot('hover-feature-card-after.png', {
      maxDiffPixelRatio: 0.02,
    })
  })
})
