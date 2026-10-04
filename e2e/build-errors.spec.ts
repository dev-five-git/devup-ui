import { expect, test } from '@playwright/test'

const API_SECTIONS: readonly string[] = [
  'Style props',
  'css / globalCss / keyframes',
  'styled',
  'Emotion css prop and ClassNames',
  'Theme reads',
  'Imports and barrels',
  'vanilla-extract',
  'StyleX',
  'Plugins and config',
]

test.describe('Build Errors reference', () => {
  // Read the exported SSR HTML without vinext's client router, at a width
  // where the docs sidebar is shown.
  test.use({
    javaScriptEnabled: false,
    viewport: { width: 1440, height: 900 },
  })

  test('serves the page heading and canonical URL', async ({ page }) => {
    const response = await page.goto('/docs/build-errors')

    expect(response?.status()).toBe(200)
    await expect(page.locator('.markdown-body h1')).toHaveText('Build Errors')
    await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
      'href',
      /^(?:https:\/\/devup-ui\.com)?\/docs\/build-errors$/,
    )
  })

  test('has a section for each API, in order', async ({ page }) => {
    await page.goto('/docs/build-errors')

    const headings = await page.locator('.markdown-body h2').allTextContents()

    expect(
      headings.filter((heading) => API_SECTIONS.includes(heading)),
      `h2 headings: ${JSON.stringify(headings)}`,
    ).toEqual(API_SECTIONS)
  })

  test('docs sidebar links the page right after the limitations page', async ({
    page,
  }) => {
    await page.goto('/docs/build-errors')

    const sidebarLink = page.locator(
      'a[href="/docs/limitations"] + a[href="/docs/build-errors"]',
    )
    await expect(sidebarLink).toBeVisible()
    await expect(sidebarLink).toHaveText('Build Errors')
  })

  test('limitations Build errors section links the page', async ({ page }) => {
    await page.goto('/docs/limitations')

    await expect(
      page.locator('h2#build-errors ~ p a[href="/docs/build-errors"]'),
    ).toHaveText('Build Errors')
  })
})
