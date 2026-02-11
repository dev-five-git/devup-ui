import { expect, test } from '@playwright/test'

import { waitForStyleSettle } from './helpers'

test.describe('Landing Page - Zero Runtime Validation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
  })

  test('no dynamically injected <style> tags after page load', async ({
    page,
  }) => {
    // Count <style> elements immediately after load
    const initialStyleCount = await page.evaluate(
      () => document.querySelectorAll('style').length,
    )

    // Interact with the page: scroll, hover, etc.
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight))
    await waitForStyleSettle(page)
    await page.evaluate(() => window.scrollTo(0, 0))
    await waitForStyleSettle(page)

    // Count again after interactions
    const afterInteractionStyleCount = await page.evaluate(
      () => document.querySelectorAll('style').length,
    )

    expect(
      afterInteractionStyleCount,
      `Style count changed from ${initialStyleCount} to ${afterInteractionStyleCount} — runtime CSS injection detected`,
    ).toBe(initialStyleCount)
  })

  test('no runtime CSS-in-JS globals are present', async ({ page }) => {
    const runtimeGlobals = await page.evaluate(() => {
      const win = window as Record<string, unknown>
      return {
        // Common CSS-in-JS runtime indicators
        __devup__: '__devup__' in win,
        __emotion__: '__EMOTION_CACHE__' in win || '__emotion_cache__' in win,
        __styled__: '__styled_components__' in win,
        __stitches__: '__stitches__' in win,
      }
    })

    expect(
      runtimeGlobals.__devup__,
      'No __devup__ runtime global should exist',
    ).toBe(false)
    expect(
      runtimeGlobals.__emotion__,
      'No Emotion runtime global should exist',
    ).toBe(false)
    expect(
      runtimeGlobals.__styled__,
      'No styled-components runtime global should exist',
    ).toBe(false)
    expect(
      runtimeGlobals.__stitches__,
      'No Stitches runtime global should exist',
    ).toBe(false)
  })

  test('all styling comes from <link rel="stylesheet"> tags', async ({
    page,
  }) => {
    const stylesheetLinks = await page.evaluate(() => {
      const links = Array.from(
        document.querySelectorAll('link[rel="stylesheet"]'),
      )
      return links.map((link) => link.getAttribute('href')).filter(Boolean)
    })

    expect(
      stylesheetLinks.length,
      'Expected stylesheets to be loaded via <link> tags',
    ).toBeGreaterThan(0)

    // Verify CSS files contain devup-ui generated styles
    // (Next.js bundles CSS into hashed chunk filenames, so check content)
    const hasDevupCss = await page.evaluate(async () => {
      const links = Array.from(
        document.querySelectorAll('link[rel="stylesheet"]'),
      )
      for (const link of links) {
        const href = link.getAttribute('href')
        if (!href) continue
        try {
          const res = await fetch(href)
          const text = await res.text()
          if (
            text.includes('--primary') ||
            text.includes('--footerBg') ||
            text.includes('--background')
          ) {
            return true
          }
        } catch {
          // skip
        }
      }
      return false
    })
    expect(hasDevupCss, 'Expected devup-ui CSS variables in stylesheets').toBe(
      true,
    )
  })

  test('no inline style attributes from CSS-in-JS runtime', async ({
    page,
  }) => {
    // devup-ui may use style attributes for CSS variables (dynamic values)
    // but should NOT inject full style objects like runtime CSS-in-JS
    const runtimeStyleElements = await page.evaluate(() => {
      const allElements = Array.from(document.querySelectorAll('[style]'))
      const suspiciousStyles: string[] = []

      for (const el of allElements) {
        const style = el.getAttribute('style') || ''
        // CSS variable assignments (--varName: value) are OK for devup-ui
        // Full property declarations (background-color:, display:, etc.) would be suspicious
        // if there are many on the same element
        const properties = style.split(';').filter((p) => p.trim())
        const nonVarProperties = properties.filter(
          (p) => !p.trim().startsWith('--'),
        )
        // Allow some inline styles (GTM noscript iframe has display:none, etc.)
        if (nonVarProperties.length > 3) {
          suspiciousStyles.push(
            `<${el.tagName.toLowerCase()}>: ${style.substring(0, 100)}`,
          )
        }
      }

      return suspiciousStyles
    })

    expect(
      runtimeStyleElements,
      `Found elements with suspicious inline styles (possible runtime CSS-in-JS): ${runtimeStyleElements.join('\n')}`,
    ).toHaveLength(0)
  })

  test('style tag count stays stable during hover interactions', async ({
    page,
  }) => {
    const initialCount = await page.evaluate(
      () => document.querySelectorAll('style').length,
    )

    // Hover over various interactive elements
    const getStarted = page.getByRole('link', { name: /Get started/i })
    if (await getStarted.isVisible()) {
      await getStarted.hover()
      await waitForStyleSettle(page)
    }

    const discord = page.getByRole('link', { name: /Join our Discord/i })
    if (await discord.isVisible()) {
      await discord.hover()
      await waitForStyleSettle(page)
    }

    // Move away
    await page.mouse.move(0, 0)
    await waitForStyleSettle(page)

    const finalCount = await page.evaluate(
      () => document.querySelectorAll('style').length,
    )

    expect(
      finalCount,
      `Style elements changed from ${initialCount} to ${finalCount} during interactions`,
    ).toBe(initialCount)
  })
})
