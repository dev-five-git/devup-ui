import { expect, test } from '@playwright/test'

import { waitForFontsReady, waitForStyleSettle } from './helpers'

test.describe('Components Pages', () => {
  test.describe('Components link availability', () => {
    test('header has Components link pointing to /components/overview', async ({
      browser,
    }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const link = page.locator('a[href="/components/overview"]').first()
      await expect(link).toBeVisible()
      await expect(link).toHaveAttribute('href', '/components/overview')
      await context.close()
    })

    test('Components link text is correct', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const link = page
        .locator('a[href="/components/overview"]')
        .filter({ hasText: 'Components' })
      await expect(link).toBeVisible()
      await context.close()
    })

    test('Components link is hidden on mobile', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 375, height: 812 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const link = page
        .locator('a[href="/components/overview"]')
        .filter({ hasText: 'Components' })
      const isVisible = await link.isVisible().catch(() => false)
      expect(isVisible).toBeFalsy()
      await context.close()
    })

    test('Components link is visible on desktop', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const link = page
        .locator('a[href="/components/overview"]')
        .filter({ hasText: 'Components' })
      await expect(link).toBeVisible()
      await context.close()
    })
  })

  test.describe('Storybook link', () => {
    test('header has Storybook link', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const link = page.locator('a[href="/storybook/index.html"]').first()
      await expect(link).toBeVisible()
      await context.close()
    })

    test('Storybook link is hidden on mobile', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 375, height: 812 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const link = page
        .locator('a[href="/storybook/index.html"]')
        .filter({ hasText: 'Storybook' })
      const isVisible = await link.isVisible().catch(() => false)
      expect(isVisible).toBeFalsy()
      await context.close()
    })
  })

  test.describe('Benchmark section (component showcase on home page)', () => {
    test('has Comparison Benchmarks section', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      await expect(page.getByText('Comparison Bechmarks').first()).toBeVisible()
      await context.close()
    })

    test('shows Devup UI benchmark card', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const benchSection = page.getByText('Comparison Bechmarks').first()
      await benchSection.scrollIntoViewIfNeeded()

      // The Devup UI card is inside a client-side animation wrapper that keeps
      // it visually hidden (opacity/transform) when JS is disabled. Verify the
      // element exists in the DOM instead.
      await expect(
        page.locator('.typo-h5', { hasText: 'Devup UI' }).first(),
      ).toBeAttached()
      await context.close()
    })

    test('shows competitor benchmarks', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const benchSection = page.getByText('Comparison Bechmarks').first()
      await benchSection.scrollIntoViewIfNeeded()
      await waitForStyleSettle(page)

      // Check that several competitor names are visible
      await expect(page.getByText('Chakra UI').first()).toBeVisible()
      await expect(page.getByText('Tailwindcss').first()).toBeVisible()
      await context.close()
    })

    test('desktop benchmark section screenshot', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const benchSection = page.getByText('Comparison Bechmarks').first()
      await benchSection.scrollIntoViewIfNeeded()
      await waitForStyleSettle(page)

      await expect(page).toHaveScreenshot(
        'components-benchmark-section-desktop.png',
        { fullPage: false },
      )
      await context.close()
    })

    test('mobile benchmark section screenshot', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 375, height: 812 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const benchSection = page.getByText('Comparison Bechmarks').first()
      await benchSection.scrollIntoViewIfNeeded()
      await waitForStyleSettle(page)

      await expect(page).toHaveScreenshot(
        'components-benchmark-section-mobile.png',
        { fullPage: false },
      )
      await context.close()
    })

    test('dark mode benchmark section screenshot', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
        colorScheme: 'dark',
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const benchSection = page.getByText('Comparison Bechmarks').first()
      await benchSection.scrollIntoViewIfNeeded()
      await waitForStyleSettle(page)

      await expect(page).toHaveScreenshot(
        'dark-components-benchmark-section-desktop.png',
        { fullPage: false },
      )
      await context.close()
    })
  })

  test.describe('Community section', () => {
    test('has Join our community section', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const community = page.getByText('Join our community').first()
      await community.scrollIntoViewIfNeeded()
      await expect(community).toBeVisible()
      await context.close()
    })

    test('has Discord link', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const discordLink = page
        .locator('a[href="https://discord.gg/8zjcGc7cWh"]')
        .first()
      await expect(discordLink).toBeVisible()
      await context.close()
    })

    test('has KakaoTalk link', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const kakaoLink = page
        .locator('a[href="https://open.kakao.com/o/giONwVAh"]')
        .first()
      await expect(kakaoLink).toBeVisible()
      await context.close()
    })
  })
})
