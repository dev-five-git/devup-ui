import { expect, test } from '@playwright/test'

import { waitForFontsReady, waitForStyleSettle } from './helpers'

/**
 * All tests use javaScriptEnabled: false because the Next.js static export
 * serves correct SSR HTML, but client-side hydration triggers a 404 under
 * the custom static server (no real Next.js router available).
 *
 * With JS disabled the raw SSR HTML renders perfectly — sidebar links,
 * doc content, and all static elements are present.
 */

test.describe('Documentation Pages', () => {
  test.describe('Docs link availability', () => {
    test('header has Docs link pointing to /docs/overview', async ({
      browser,
    }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const docsLink = page.locator('a[href="/docs/overview"]').first()
      await expect(docsLink).toBeVisible()
      await expect(docsLink).toHaveAttribute('href', '/docs/overview')

      await context.close()
    })

    test('Get Started button links to docs overview', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const getStarted = page.locator('a[href="/docs/overview"]').filter({
        hasText: 'Get started',
      })
      await expect(getStarted).toBeVisible()

      await context.close()
    })

    test('Docs link is hidden on mobile', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 375, height: 812 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      // The header nav link should not be visible on mobile
      const docsNavLink = page
        .locator('a[href="/docs/overview"]')
        .filter({ hasText: 'Docs' })
      const isVisible = await docsNavLink.isVisible().catch(() => false)
      expect(isVisible).toBeFalsy()

      await context.close()
    })

    test('Docs link is visible on desktop', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const docsNavLink = page
        .locator('a[href="/docs/overview"]')
        .filter({ hasText: 'Docs' })
      await expect(docsNavLink).toBeVisible()

      await context.close()
    })
  })

  test.describe('Documentation content presence on home page', () => {
    test('home page mentions key docs concepts', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      // The home page should mention Zero Runtime, which is a key concept
      await expect(page.getByText('Zero Runtime').first()).toBeVisible()

      await context.close()
    })

    test('home page has Features section', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      await expect(page.getByText('Features').first()).toBeVisible()

      await context.close()
    })

    test('home page has Type Safety feature', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      await expect(page.getByText('Type Safety').first()).toBeVisible()

      await context.close()
    })

    test('home page has Figma Plugin feature', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const figmaPlugin = page.getByText('Figma Plugin').first()
      await figmaPlugin.scrollIntoViewIfNeeded()
      await expect(figmaPlugin).toBeVisible()

      await context.close()
    })
  })

  test.describe('Docs-related visual regression', () => {
    test('desktop features section screenshot', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const features = page.getByText('Features').first()
      await features.scrollIntoViewIfNeeded()
      await waitForStyleSettle(page)

      await expect(page).toHaveScreenshot('docs-features-section-desktop.png', {
        fullPage: false,
      })

      await context.close()
    })

    test('mobile features section screenshot', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 375, height: 812 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const features = page.getByText('Features').first()
      await features.scrollIntoViewIfNeeded()
      await waitForStyleSettle(page)

      await expect(page).toHaveScreenshot('docs-features-section-mobile.png', {
        fullPage: false,
      })

      await context.close()
    })

    test('dark mode features section screenshot', async ({ browser }) => {
      const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
        colorScheme: 'dark',
      })
      const page = await context.newPage()
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await page.evaluate(() =>
        document.documentElement.setAttribute('data-theme', 'dark'),
      )
      await waitForStyleSettle(page)

      const features = page.getByText('Features').first()
      await features.scrollIntoViewIfNeeded()
      await waitForStyleSettle(page)

      await expect(page).toHaveScreenshot(
        'dark-docs-features-section-desktop.png',
        { fullPage: false },
      )

      await context.close()
    })
  })

  test.describe('Get Started CTA', () => {
    test('Get Started button is visible on desktop', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 1440, height: 900 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const getStarted = page.getByText('Get started').first()
      await expect(getStarted).toBeVisible()

      await context.close()
    })

    test('Get Started button is visible on mobile', async ({ browser }) => {
      const context = await browser.newContext({
        javaScriptEnabled: false,
        viewport: { width: 375, height: 812 },
      })
      const page = await context.newPage()
      await page.goto('/')
      await waitForFontsReady(page)

      const getStarted = page.getByText('Get started').first()
      await expect(getStarted).toBeVisible()

      await context.close()
    })
  })
})
