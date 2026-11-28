import { expect, test } from '@playwright/test'

import { getUntranslatedRoutes } from './exported-routes'

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
    page.getByRole('link', { name: '이 페이지 편집' }),
  ).toHaveAttribute(
    'href',
    /src\/app\/\(detail\)\/docs\/overview\/page\.ko\.mdx$/u,
  )
  await expect(
    page.getByRole('navigation', { name: '주요 메뉴' }).getByText('문서'),
  ).toBeVisible()
  await expect(
    page.getByRole('button', { name: '색상 테마 전환' }),
  ).toBeVisible()
  await expect(page.getByPlaceholder('문서 검색...')).toBeVisible()
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://devup-ui.com/ko/docs/overview',
  )

  await page
    .getByRole('link', { name: '빌드 오류', exact: true })
    .first()
    .click()
  await expect(page).toHaveURL(/\/ko\/docs\/build-errors$/u)
  await expect(page.locator('html')).toHaveAttribute('lang', 'ko')
  await expect(
    page.getByRole('heading', { name: '빌드 오류', exact: true }),
  ).toBeVisible()
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://devup-ui.com/ko/docs/build-errors',
  )
  await expect(
    page.getByRole('link', { name: 'Edit this page' }),
  ).toHaveAttribute(
    'href',
    /src\/app\/\(detail\)\/docs\/build-errors\/page\.ko\.mdx$/u,
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
  await page.goto('/ko/docs/overview?search=1&query=Button')
  await expect(
    page
      .locator('a[href="/ko/components/button"]')
      .filter({ hasText: 'Button' })
      .last(),
  ).toBeVisible()
  await page.goto('/ko/docs/overview?search=1&query=zzzz-no-match')
  await expect(page.getByText('검색 결과가 없습니다')).toBeVisible()
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
  await page.goto('/ko/docs/build-errors')
  await expect(page).toHaveURL(/\/ko\/docs\/build-errors$/u)
  await expect(
    page.getByRole('heading', { name: '빌드 오류', exact: true }),
  ).toBeVisible()
  await page.goto('/ko')
  await expect(page).toHaveURL('http://localhost:3099/ko')
  await expect(page.locator('html')).toHaveAttribute('lang', 'ko')
  await expect(page.getByText('시작하기', { exact: true })).toBeVisible()
  await page.goto('/ko/components/button')
  await expect(page).toHaveURL(/\/ko\/components\/button$/u)
  await expect(page.locator('html')).toHaveAttribute('lang', 'ko')
  await expect(
    page.getByRole('heading', { name: 'Button', exact: true }).first(),
  ).toBeVisible()
  const response = await page.goto('/ko/docs/does-not-exist')
  expect(response?.status()).toBe(404)
  const unsupportedLocale = await page.goto('/fr/docs/overview')
  expect(unsupportedLocale?.status()).toBe(404)
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

test('Korean home and showcase translate their copy and keep English unchanged', async ({
  page,
}) => {
  await page.route('**/api.github.com/repos/dev-five-git/devup-ui', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ stargazers_count: 1234 }),
    }),
  )
  await page.setViewportSize({ width: 1920, height: 1080 })
  await page.goto('/')
  await expect(
    page.getByRole('heading', { name: 'Comparison Benchmarks' }),
  ).toBeVisible()
  await page.goto('/ko')
  await expect(page.locator('html')).toHaveAttribute('lang', 'ko')
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://devup-ui.com/ko',
  )
  await expect(
    page.getByRole('heading', { name: '벤치마크 비교' }),
  ).toBeVisible()
  await expect(page.getByRole('heading', { name: '주요 기능' })).toBeVisible()
  await expect(
    page.getByRole('heading', { name: '커뮤니티에 참여하세요' }),
  ).toBeVisible()
  await page.getByRole('link', { name: '시작하기' }).click()
  await expect(page).toHaveURL(/\/ko\/docs\/overview$/u)

  await page.goto('/ko/showcase')
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://devup-ui.com/ko/showcase',
  )
  await expect(page.getByRole('heading', { name: '쇼케이스' })).toBeVisible()
  await page.getByRole('button', { name: '모바일' }).click()
  await expect(page.getByText('NH Allone Bank')).toBeVisible()
  await expect(page.getByText('Braillify')).toHaveCount(0)
})

test('Korean component pages translate their prose and keep API names', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1920, height: 1080 })
  await page.goto('/ko/components/overview')
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://devup-ui.com/ko/components/overview',
  )
  await expect(
    page.getByRole('heading', { name: 'Devup UI 컴포넌트', exact: true }),
  ).toBeVisible()
  await expect(
    page.getByRole('heading', { name: '폼', exact: true }),
  ).toBeVisible()
  await page.locator('main a[href="/ko/components/button"]').click()
  await expect(page).toHaveURL(/\/ko\/components\/button$/u)
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    'href',
    'https://devup-ui.com/ko/components/button',
  )
  await expect(
    page.getByRole('heading', { name: 'Button', exact: true }),
  ).toBeVisible()
  await expect(
    page.getByText('컴포넌트는 사용자 상호작용을 처리하는 데 사용합니다.'),
  ).toBeVisible()
  await expect(
    page.getByRole('heading', { name: '예제', exact: true }),
  ).toBeVisible()
  await expect(page.getByRole('columnheader', { name: '기본값' })).toBeVisible()
  await expect(page.getByText('코드 보기').first()).toBeVisible()

  await page.goto('/ko/components/tooltip')
  await expect(page.getByText('준비 중', { exact: true })).toBeVisible()
  await page.goto('/components/tooltip')
  await expect(page.getByText('Coming Soon', { exact: true })).toBeVisible()
})

test('untranslated Korean routes fall back to English without JavaScript', async ({
  browser,
}) => {
  const untranslated = getUntranslatedRoutes()
  test.skip(untranslated.length === 0, 'Every page has a Korean translation')
  const context = await browser.newContext({ javaScriptEnabled: false })
  const page = await context.newPage()
  for (const route of untranslated) {
    const english = route.slice('/ko'.length) || '/'
    await page.goto(route)
    await expect(page).toHaveURL(new URL(english, 'http://localhost:3099').href)
    await expect(page.locator('html')).toHaveAttribute('lang', 'en')
  }
  await context.close()
})
