import { expect, test } from '@playwright/test'

test.describe('Landing Page - Build Integrity', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('page loads without console errors', async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => {
      errors.push(error.message)
    })

    // Re-navigate to capture errors from initial load
    await page.goto('/')
    await page.waitForLoadState('networkidle')

    expect(errors, 'Page should load without JS errors').toHaveLength(0)
  })

  test('CSS stylesheets are present in <head>', async ({ page }) => {
    const stylesheetCount = await page.locator('link[rel="stylesheet"]').count()
    expect(
      stylesheetCount,
      'Expected at least one CSS stylesheet link in <head>',
    ).toBeGreaterThan(0)
  })

  test('CSS files contain devup-ui generated classes', async ({ page }) => {
    // devup-ui generates CSS with short class names (a, b, c, ..., aa, ab, ...)
    // Next.js bundles them into hashed chunk filenames, so we check CSS content
    const hasDevupStyles = await page.evaluate(async () => {
      const links = Array.from(
        document.querySelectorAll('link[rel="stylesheet"]'),
      )
      for (const link of links) {
        const href = link.getAttribute('href')
        if (!href) continue
        try {
          const res = await fetch(href)
          const text = await res.text()
          // devup-ui uses CSS custom properties like --primary, --text, --background
          // and generates short class selectors
          if (
            text.includes('--primary') ||
            text.includes('--footerBg') ||
            text.includes('--background')
          ) {
            return true
          }
        } catch {
          // skip fetch errors
        }
      }
      return false
    })

    expect(
      hasDevupStyles,
      'Expected CSS files to contain devup-ui generated styles (CSS variables like --primary, --footerBg)',
    ).toBe(true)
  })

  test('no 404 errors on CSS or JS resources', async ({ page }) => {
    const failedRequests: string[] = []

    page.on('response', (response) => {
      const url = response.url()
      if (
        response.status() === 404 &&
        (url.endsWith('.css') || url.endsWith('.js'))
      ) {
        failedRequests.push(`${response.status()} ${url}`)
      }
    })

    await page.goto('/')
    await page.waitForLoadState('networkidle')

    expect(
      failedRequests,
      `Found 404 errors for resources: ${failedRequests.join(', ')}`,
    ).toHaveLength(0)
  })

  test('page has actual visible content (not blank)', async ({ page }) => {
    // Check for the hero text that should always be present
    const heroText = page.getByText('Zero Config')
    await expect(heroText.first()).toBeVisible()

    // Check for the "Features" section heading
    const featuresHeading = page.getByText('Features')
    await expect(featuresHeading.first()).toBeVisible()

    // Check that body has non-trivial content
    const bodyText = await page.evaluate(
      () => document.body.innerText.trim().length,
    )
    expect(
      bodyText,
      'Page body should have substantial text content',
    ).toBeGreaterThan(100)
  })

  test('page title is set correctly', async ({ page }) => {
    await expect(page).toHaveTitle(/Devup UI/)
  })
})
