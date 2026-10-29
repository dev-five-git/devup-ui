import { expect, test } from '@playwright/test'

test('English remains the default and Korean overview is translated', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1920, height: 1080 })
  await page.goto('/docs/overview')
  await expect(page.locator('html')).toHaveAttribute('lang', 'en')
  await expect(
    page.getByRole('heading', { name: 'What is Devup UI?', exact: true }),
  ).toBeVisible()
  await page.getByRole('link', { name: 'Switch to Korean' }).click()
  await expect(page).toHaveURL(/\/ko\/docs\/overview$/u)
  await expect(page.locator('html')).toHaveAttribute('lang', 'ko')
  await expect(page.getByRole('link', { name: 'Storybook' })).toHaveAttribute(
    'href',
    '/storybook/index.html',
  )
  await expect(
    page.getByRole('heading', { name: 'Devup UI란?', exact: true }),
  ).toBeVisible()
  await expect(
    page.getByRole('link', { name: 'Edit this page' }),
  ).toHaveAttribute('href', /src\/app\/ko\/docs\/overview\/page.mdx$/u)
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://devup-ui.com/ko/docs/overview',
  )

  await page
    .getByRole('link', { name: 'Installation', exact: true })
    .first()
    .click()
  await expect(page).toHaveURL(/(?<!ko)\/docs\/installation$/u)
  await expect(page.locator('html')).toHaveAttribute('lang', 'en')
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://devup-ui.com/docs/installation',
  )
  await expect(
    page.getByRole('link', { name: 'Edit this page' }),
  ).toHaveAttribute(
    'href',
    /src\/app\/\(detail\)\/docs\/installation\/page.mdx$/u,
  )
})

test('language switch preserves queries and fragments', async ({ page }) => {
  await page.goto('/ko/docs/overview?example=1#devup-ui란')
  await page.getByRole('link', { name: 'Switch to English' }).click()
  await expect(page).toHaveURL(/\/docs\/overview\?example=1#devup-ui/u)
  await expect(page.locator('html')).toHaveAttribute('lang', 'en')
})

test('Korean search finds translated content and English fallback pages', async ({
  page,
}) => {
  await page.goto('/ko/docs/overview?search=1&query=전처리기')
  await expect(
    page
      .locator('a[href="/ko/docs/overview"]')
      .filter({ hasText: 'Devup UI란?' }),
  ).toBeVisible()
  await page.goto('/ko/docs/overview?search=1&query=Installation')
  await expect(
    page
      .locator('a[href="/ko/docs/installation"]')
      .filter({ hasText: 'Installation' })
      .last(),
  ).toBeVisible()
})

test('Korean routes render translated or English content without JavaScript', async ({
  browser,
}) => {
  const context = await browser.newContext({ javaScriptEnabled: false })
  const page = await context.newPage()
  await page.goto('/ko/docs/overview')
  await expect(
    page.getByRole('heading', { name: 'Devup UI란?', exact: true }),
  ).toBeVisible()
  await page.goto('/ko/docs/installation')
  await expect(page).toHaveURL(/(?<!ko)\/docs\/installation$/u)
  await expect(page.locator('html')).toHaveAttribute('lang', 'en')
  await page.goto('/ko')
  await expect(page).toHaveURL('http://localhost:3099/')
  await expect(page.locator('html')).toHaveAttribute('lang', 'en')
  await page.goto('/ko/components/button')
  await expect(page).toHaveURL(/(?<!ko)\/components\/button$/u)
  await expect(
    page.getByRole('heading', { name: 'Button', exact: true }).first(),
  ).toBeVisible()
  const response = await page.goto('/ko/docs/does-not-exist')
  expect(response?.status()).toBe(404)
  await context.close()
})

test('mobile menu exposes the language switcher', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 812 })
  await page.goto('/ko/docs/overview?menu=1')
  await expect(
    page.getByRole('link', { name: 'Switch to English' }),
  ).toBeVisible()
  await expect(
    page.locator('a[href="/ko/docs/installation"]').first(),
  ).toBeVisible()
  await expect(page.getByRole('link', { name: 'Storybook' })).toHaveAttribute(
    'href',
    '/storybook/index.html',
  )
})

test('language navigation fits tablet screens', async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 900 })
  for (const path of ['/', '/docs/overview', '/ko/docs/overview']) {
    await page.goto(path)
    const width = await page.evaluate(
      () => document.documentElement.scrollWidth,
    )
    expect(width).toBeLessThanOrEqual(1024)
  }
})
