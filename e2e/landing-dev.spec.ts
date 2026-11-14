import { expect, test as base } from '@playwright/test'

const test = base.extend<{ browserErrors: string[] }>({
  browserErrors: [
    async ({ page }, use, testInfo) => {
      const errors: string[] = []
      page.on('console', (message) => {
        if (message.type() === 'error') {
          errors.push(`${page.url()}: ${message.text()}`)
        }
      })
      page.on('pageerror', (error) => {
        errors.push(`${page.url()}: ${error.stack ?? error.message}`)
      })
      await use(errors)
      await testInfo.attach('browser-errors', {
        body: JSON.stringify(errors, null, 2),
        contentType: 'application/json',
      })
      expect(errors, 'Browser console and page errors').toEqual([])
    },
    { auto: true },
  ],
})

test.beforeEach(async ({ page }) => {
  await page.route('**/api.github.com/repos/dev-five-git/devup-ui', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ stargazers_count: 1234 }),
    }),
  )
})

const routes = [
  ['/', '/'],
  ['/docs/overview', '/docs/overview'],
  ['/ko', '/'],
  ['/ko/docs/overview', '/ko/docs/overview'],
  ['/team', '/team'],
  ['/components/button', '/components/button'],
  ['/ko/docs/features', '/docs/features'],
] as const

for (const [route, target] of routes) {
  test(`renders ${route} without a dev overlay or lazy import failure`, async ({
    page,
  }) => {
    const response = await page.goto(route, { waitUntil: 'networkidle' })

    expect(response?.status()).toBe(200)
    await expect(page).toHaveURL(`http://localhost:3098${target}`)
    await expect(page.locator('main')).toBeVisible()
    await expect(page.locator('main')).not.toBeEmpty()
    await expect(
      page.locator('vite-error-overlay, nextjs-portal [data-nextjs-dialog]'),
    ).toHaveCount(0)
    const theme = page.getByRole('button', { name: 'Toggle color theme' })
    const initialTheme = await page.locator('html').getAttribute('data-theme')
    await theme.click()
    await expect(page.locator('html')).toHaveAttribute(
      'data-theme',
      initialTheme === 'dark' ? 'light' : 'dark',
    )
  })
}

test('header links and browser back retain a rendered page', async ({
  page,
}) => {
  await page.goto('/', { waitUntil: 'networkidle' })
  const navigation = page.getByRole('navigation', {
    name: 'Primary navigation',
  })

  await navigation.getByRole('link', { name: 'Docs', exact: true }).click()
  await expect(page).toHaveURL(/\/docs\/overview$/u)
  await navigation
    .getByRole('link', { name: 'Components', exact: true })
    .click()
  await expect(page).toHaveURL(/\/components\/overview$/u)
  await navigation.getByRole('link', { name: 'Team', exact: true }).click()
  await expect(page).toHaveURL(/\/team$/u)
  await expect(page.locator('main')).toBeVisible()
  await page.goBack()
  await expect(page).toHaveURL(/\/components\/overview$/u)
  await expect(page.locator('main')).toBeVisible()
})

test('language switcher works in both directions after client navigation', async ({
  page,
}) => {
  await page.goto('/', { waitUntil: 'networkidle' })
  await page
    .getByRole('navigation', { name: 'Primary navigation' })
    .getByRole('link', { name: 'Docs', exact: true })
    .click()
  await expect(page).toHaveURL(/\/docs\/overview$/u)
  await expect(
    page.getByRole('heading', { name: 'What is Devup UI?', exact: true }),
  ).toBeVisible()

  await page.getByRole('link', { name: 'Switch to Korean' }).click()
  await expect(page).toHaveURL(/\/ko\/docs\/overview$/u)
  await expect(page.locator('html')).toHaveAttribute('lang', 'ko')
  await expect(
    page.getByRole('heading', { name: 'Devup UI란?', exact: true }),
  ).toBeVisible()
  await page.getByRole('link', { name: 'Switch to English' }).click()
  await expect(page).toHaveURL(/(?<!ko)\/docs\/overview$/u)
  await expect(page.locator('html')).toHaveAttribute('lang', 'en')
})

for (const locale of ['en', 'ko'] as const) {
  const prefix = locale === 'ko' ? '/ko' : ''

  test(`${locale} docs navigation and back preserve translated or fallback content`, async ({
    page,
  }) => {
    await page.goto(`${prefix}/docs/overview`, { waitUntil: 'networkidle' })

    await page
      .getByRole('link', { name: 'Features', exact: true })
      .first()
      .click()
    await expect(page).toHaveURL(/(?<!ko)\/docs\/features$/u)
    await expect(page.locator('main')).toBeVisible()
    await page.goBack()
    await expect(page).toHaveURL(`http://localhost:3098${prefix}/docs/overview`)
    await expect(page.locator('html')).toHaveAttribute('lang', locale)
  })

  test(`${locale} search opens, updates its query and navigates to a result`, async ({
    page,
  }) => {
    await page.goto(`${prefix}/docs/overview`, { waitUntil: 'networkidle' })

    await page.locator('input[readonly]').click()
    const input = page.locator('input:not([readonly])')
    await expect(input).toBeVisible()
    await input.fill(locale === 'ko' ? '전처리기' : 'What is Devup UI?')
    const result = page
      .locator(`a[href="${prefix}/docs/overview"]`)
      .filter({
        hasText: locale === 'ko' ? 'Devup UI란?' : 'What is Devup UI?',
      })
      .last()
    await expect(result).toBeVisible()
    await result.click()
    await expect(page).toHaveURL(`http://localhost:3098${prefix}/docs/overview`)
    await expect(input).toHaveCount(0)
    await expect(page.locator('html')).toHaveAttribute('lang', locale)
  })
}
