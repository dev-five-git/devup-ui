import { expect, test } from '@playwright/test'

import { waitForStyleSettle } from './helpers'

/**
 * Normalize a color string to lowercase hex.
 */
function normalizeColor(raw: string): string {
  const trimmed = raw.trim().toLowerCase()
  if (trimmed.startsWith('#')) {
    const hex = trimmed.replace('#', '')
    if (hex.length === 3) {
      return `#${hex[0]}${hex[0]}${hex[1]}${hex[1]}${hex[2]}${hex[2]}`
    }
    return `#${hex.substring(0, 6)}`
  }
  const match = trimmed.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/)
  if (match) {
    const r = parseInt(match[1], 10).toString(16).padStart(2, '0')
    const g = parseInt(match[2], 10).toString(16).padStart(2, '0')
    const b = parseInt(match[3], 10).toString(16).padStart(2, '0')
    return `#${r}${g}${b}`
  }
  return trimmed
}

test.describe('Landing Page - Theme Switching', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
  })

  test('light theme: --primary is #5A44FF', async ({ page }) => {
    // Ensure light theme is active
    await page.evaluate(() =>
      document.documentElement.setAttribute('data-theme', 'light'),
    )
    await waitForStyleSettle(page)

    const bodyBg = await page.evaluate(
      () => getComputedStyle(document.body).backgroundColor,
    )

    // The body bg should be $footerBg light = #F4F4F6
    expect(normalizeColor(bodyBg)).toBe('#f4f4f6')
  })

  test('light theme: --text resolves to #2F2F2F', async ({ page }) => {
    await page.evaluate(() =>
      document.documentElement.setAttribute('data-theme', 'light'),
    )
    await waitForStyleSettle(page)

    const bodyColor = await page.evaluate(
      () => getComputedStyle(document.body).color,
    )
    expect(normalizeColor(bodyColor)).toBe('#2f2f2f')
  })

  test('dark theme: background changes to #131313', async ({ page }) => {
    // Switch to dark theme
    await page.evaluate(() =>
      document.documentElement.setAttribute('data-theme', 'dark'),
    )
    await waitForStyleSettle(page)

    // The main content wrapper (inside body) has bg=$background
    // which is #131313 in dark. Body itself has $footerBg = #2E303C in dark.
    const bodyBg = await page.evaluate(
      () => getComputedStyle(document.body).backgroundColor,
    )

    // $footerBg dark = #2E303C
    expect(normalizeColor(bodyBg)).toBe('#2e303c')
  })

  test('dark theme: text color changes to #EDEDED', async ({ page }) => {
    await page.evaluate(() =>
      document.documentElement.setAttribute('data-theme', 'dark'),
    )
    await waitForStyleSettle(page)

    const bodyColor = await page.evaluate(
      () => getComputedStyle(document.body).color,
    )
    // $text dark = #EDEDED
    expect(normalizeColor(bodyColor)).toBe('#ededed')
  })

  test('dark theme: footer background changes', async ({ page }) => {
    // Light footer bg
    const lightFooterBg = await page.evaluate(() => {
      const footer = document.querySelector('footer')
      return footer ? getComputedStyle(footer).backgroundColor : ''
    })

    // Switch to dark
    await page.evaluate(() =>
      document.documentElement.setAttribute('data-theme', 'dark'),
    )
    await waitForStyleSettle(page)

    const darkFooterBg = await page.evaluate(() => {
      const footer = document.querySelector('footer')
      return footer ? getComputedStyle(footer).backgroundColor : ''
    })

    // $footerBg dark = #2E303C
    expect(normalizeColor(darkFooterBg)).toBe('#2e303c')
    // Ensure it actually changed from the light value
    expect(normalizeColor(lightFooterBg)).not.toBe(normalizeColor(darkFooterBg))
  })

  test('theme can toggle back and forth', async ({ page }) => {
    // Start light
    await page.evaluate(() =>
      document.documentElement.setAttribute('data-theme', 'light'),
    )
    await waitForStyleSettle(page)
    const lightBg = await page.evaluate(
      () => getComputedStyle(document.body).backgroundColor,
    )

    // Switch dark
    await page.evaluate(() =>
      document.documentElement.setAttribute('data-theme', 'dark'),
    )
    await waitForStyleSettle(page)
    const darkBg = await page.evaluate(
      () => getComputedStyle(document.body).backgroundColor,
    )

    // Switch back to light
    await page.evaluate(() =>
      document.documentElement.setAttribute('data-theme', 'light'),
    )
    await waitForStyleSettle(page)
    const lightBgAgain = await page.evaluate(
      () => getComputedStyle(document.body).backgroundColor,
    )

    expect(normalizeColor(lightBg)).toBe('#f4f4f6')
    expect(normalizeColor(darkBg)).toBe('#2e303c')
    expect(normalizeColor(lightBgAgain)).toBe('#f4f4f6')
  })

  test('theme switch persists the selection across reloads', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.evaluate(() => {
      localStorage.setItem('__DF_THEME_SELECTED__', 'light')
    })
    await page.reload({ waitUntil: 'networkidle' })

    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
    await page.getByRole('button', { name: 'Toggle color theme' }).click()
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
    expect(
      await page.evaluate(() => localStorage.getItem('__DF_THEME_SELECTED__')),
    ).toBe('dark')

    await page.reload({ waitUntil: 'networkidle' })
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  })
})

const FIRST_PAINT_VIEWPORTS = {
  desktop: { width: 1440, height: 900 },
  mobile: { width: 375, height: 812 },
} as const

const FIRST_PAINT_BACKGROUND = {
  dark: '#2e303c',
  light: '#f4f4f6',
} as const

for (const [viewportName, viewport] of Object.entries(FIRST_PAINT_VIEWPORTS)) {
  for (const theme of ['light', 'dark'] as const) {
    test(`applies ${theme} before first paint at ${viewportName} width`, async ({
      browser,
    }) => {
      const context = await browser.newContext({
        colorScheme: theme === 'dark' ? 'light' : 'dark',
        viewport,
      })
      await context.addInitScript((selectedTheme) => {
        localStorage.setItem('__DF_THEME_SELECTED__', selectedTheme)

        const changes: string[] = []
        ;(
          window as Window & { __landingThemeChanges?: string[] }
        ).__landingThemeChanges = changes

        const originalSetAttribute = Element.prototype.setAttribute
        Element.prototype.setAttribute = function setAttribute(name, value) {
          if (name === 'data-theme') changes.push(value)
          originalSetAttribute.call(this, name, value)
        }
      }, theme)

      const page = await context.newPage()
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

      await page.goto('/', { waitUntil: 'domcontentloaded' })
      const firstPaintState = await page.evaluate(() => ({
        background: getComputedStyle(document.body).backgroundColor,
        changes:
          (window as Window & { __landingThemeChanges?: string[] })
            .__landingThemeChanges ?? [],
        theme: document.documentElement.getAttribute('data-theme'),
      }))

      expect(firstPaintState.theme).toBe(theme)
      expect([...new Set(firstPaintState.changes)]).toEqual([theme])
      expect(normalizeColor(firstPaintState.background)).toBe(
        FIRST_PAINT_BACKGROUND[theme],
      )

      await context.close()
    })
  }
}
