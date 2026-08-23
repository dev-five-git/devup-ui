import { expect, test } from '@playwright/test'

const MAIN_ROUTES = [
  '/',
  '/docs/overview',
  '/components/overview',
  '/showcase',
  '/team',
] as const

const VIEWPORTS = {
  desktop: { width: 1440, height: 900 },
  mobile: { width: 375, height: 812 },
} as const

for (const [viewportName, viewport] of Object.entries(VIEWPORTS)) {
  for (const theme of ['light', 'dark'] as const) {
    test(`main pages have landmarks and ordered headings in ${theme} mode at ${viewportName} width`, async ({
      browser,
    }) => {
      const context = await browser.newContext({ viewport })
      await context.addInitScript((selectedTheme) => {
        localStorage.setItem('__DF_THEME_SELECTED__', selectedTheme)
      }, theme)
      const page = await context.newPage()

      for (const route of MAIN_ROUTES) {
        await page.goto(route, { waitUntil: 'networkidle' })

        await expect(page.locator('header'), `${route} header`).toHaveCount(1)
        await expect(page.locator('main'), `${route} main`).toHaveCount(1)
        await expect(page.locator('footer'), `${route} footer`).toHaveCount(1)

        const headings = await page
          .locator('main h1, main h2, main h3, main h4, main h5, main h6')
          .evaluateAll((elements) =>
            elements.map((element) => ({
              level: Number(element.tagName.slice(1)),
              text: element.textContent?.trim() ?? '',
            })),
          )

        expect(headings.length, `${route} has headings`).toBeGreaterThan(0)
        expect(
          headings.every(({ text }) => text.length > 0),
          `${route} has no empty headings`,
        ).toBe(true)

        const skippedHeading = headings.find(
          ({ level }, index) =>
            index > 0 && level > headings[index - 1].level + 1,
        )
        expect(
          skippedHeading,
          `${route} skips a heading level: ${JSON.stringify(headings)}`,
        ).toBeUndefined()
      }

      await context.close()
    })
  }
}
