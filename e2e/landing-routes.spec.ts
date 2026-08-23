import { expect, test } from '@playwright/test'

import {
  EXPECTED_EXPORTED_HTML_FILES,
  EXPECTED_EXPORTED_ROUTES,
  getExportedHtmlFiles,
  getExportedRoutes,
  getLocalAssetPaths,
} from './exported-routes'

const THEME_BACKGROUND = {
  dark: 'rgb(46, 48, 60)',
  light: 'rgb(244, 244, 246)',
} as const

const VIEWPORTS = {
  desktop: { width: 1440, height: 900 },
  mobile: { width: 375, height: 812 },
} as const

test.describe('Landing static export routes', () => {
  test('matches the complete committed route manifest', () => {
    expect(getExportedHtmlFiles()).toEqual(EXPECTED_EXPORTED_HTML_FILES)
    expect(getExportedRoutes()).toEqual([...EXPECTED_EXPORTED_ROUTES].sort())
  })

  test('serves a real 404 response for an unknown route', async ({
    request,
  }) => {
    const response = await request.get('/route-that-does-not-exist')

    expect(response.status()).toBe(404)
    expect(await response.text()).toContain('This page could not be found')
  })

  test('serves every local asset referenced by exported HTML and CSS', async ({
    request,
  }) => {
    // This sweep shares one single-threaded static server with the route-render
    // tests, which walk 63 routes each to `networkidle`. Under that contention a
    // serial request per asset starved past the 60s default (it takes ~4s
    // alone), so fetch through a bounded pool AND state the real budget.
    test.setTimeout(180_000)

    const assetPaths = getLocalAssetPaths()
    const failures: string[] = []
    const CONCURRENCY = 8
    let cursor = 0

    async function drain(): Promise<void> {
      while (cursor < assetPaths.length) {
        const assetPath = assetPaths[cursor++]
        const response = await request.get(assetPath)
        if (response.status() >= 400) {
          failures.push(`${response.status()} ${assetPath}`)
        }
      }
    }

    await Promise.all(
      Array.from({ length: Math.min(CONCURRENCY, assetPaths.length) }, drain),
    )

    expect(
      failures.sort(),
      `Broken local assets:\n${failures.join('\n')}`,
    ).toEqual([])
  })

  for (const [viewportName, viewport] of Object.entries(VIEWPORTS)) {
    for (const theme of Object.keys(THEME_BACKGROUND) as Array<
      keyof typeof THEME_BACKGROUND
    >) {
      test(`renders every route in ${theme} mode at ${viewportName} width`, async ({
        browser,
      }) => {
        test.setTimeout(300_000)

        const context = await browser.newContext({
          colorScheme: theme,
          viewport,
        })
        await context.addInitScript((selectedTheme) => {
          localStorage.setItem('__DF_THEME_SELECTED__', selectedTheme)
        }, theme)

        const page = await context.newPage()
        let activeRoute = '/'
        const consoleErrors: string[] = []
        const notFoundResponses: string[] = []

        page.on('console', (message) => {
          if (message.type() === 'error') {
            consoleErrors.push(`${activeRoute}: ${message.text()}`)
          }
        })
        page.on('pageerror', (error) => {
          consoleErrors.push(`${activeRoute}: ${error.message}`)
        })
        page.on('response', (response) => {
          if (response.status() === 404) {
            notFoundResponses.push(`${activeRoute}: ${response.url()}`)
          }
        })

        await page.route(
          '**/api.github.com/repos/dev-five-git/devup-ui',
          (route) =>
            route.fulfill({
              status: 200,
              contentType: 'application/json',
              body: JSON.stringify({ stargazers_count: 1234 }),
            }),
        )
        await page.route('**/www.googletagmanager.com/**', (route) =>
          route.fulfill({
            status: 200,
            contentType: 'application/javascript',
            body: '',
          }),
        )

        for (const route of getExportedRoutes()) {
          activeRoute = route
          const response = await page.goto(route, { waitUntil: 'load' })

          expect(response?.status(), `${route} document status`).toBe(200)
          await expect(
            page.locator('html'),
            `${route} language`,
          ).toHaveAttribute('lang', 'en')
          await expect(page.locator('html'), `${route} theme`).toHaveAttribute(
            'data-theme',
            theme,
          )
          await expect(
            page.locator('main'),
            `${route} main landmark`,
          ).toBeVisible()

          const renderedState = await page.evaluate(() => ({
            background: getComputedStyle(document.body).backgroundColor,
            mainText: document.querySelector('main')?.textContent?.trim() ?? '',
            viewportWidth: window.innerWidth,
          }))
          expect(renderedState.background, `${route} theme background`).toBe(
            THEME_BACKGROUND[theme],
          )
          expect(
            renderedState.mainText.length,
            `${route} rendered content`,
          ).toBeGreaterThan(0)
          expect(renderedState.viewportWidth, `${route} viewport`).toBe(
            viewport.width,
          )

          await expect(page).toHaveTitle(/\S+/)
          await expect(
            page.locator('meta[name="description"]'),
          ).toHaveAttribute('content', /\S+/)
          await expect(
            page.locator('meta[property="og:title"]'),
          ).toHaveAttribute('content', /\S+/)
          await expect(
            page.locator('meta[property="og:description"]'),
          ).toHaveAttribute('content', /\S+/)
          await expect(
            page.locator('meta[property="og:image"]'),
          ).toHaveAttribute('content', /\S+/)
          const canonicalHref = await page
            .locator('link[rel="canonical"]')
            .getAttribute('href')
          expect(canonicalHref, `${route} canonical`).not.toBeNull()
          expect(
            new URL(canonicalHref ?? '', 'https://devup-ui.com').href,
            `${route} canonical`,
          ).toBe(new URL(route, 'https://devup-ui.com').href)
          await expect(page.locator('link[rel~="icon"]')).toHaveAttribute(
            'href',
            '/favicon.ico',
          )
          expect(
            await page.locator('link[rel="stylesheet"]').count(),
            `${route} static stylesheets`,
          ).toBeGreaterThan(0)

          await page.evaluate(
            () =>
              new Promise<void>((resolve) => {
                requestAnimationFrame(() =>
                  requestAnimationFrame(() => resolve()),
                )
              }),
          )
        }

        expect(
          consoleErrors,
          `Console errors:\n${consoleErrors.join('\n')}`,
        ).toEqual([])
        expect(
          notFoundResponses,
          `404 responses:\n${notFoundResponses.join('\n')}`,
        ).toEqual([])

        await context.close()
      })
    }
  }
})
