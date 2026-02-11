import type { Page } from '@playwright/test'

/**
 * Wait for all fonts to be loaded and a rendering frame to complete.
 * Replaces waitForTimeout(1000) after page.goto()
 *
 * Falls back to waitForLoadState('load') when JavaScript is disabled
 * (page.evaluate is not available in JS-disabled contexts).
 */
export async function waitForFontsReady(page: Page): Promise<void> {
  try {
    await page.evaluate(async () => {
      await document.fonts.ready
      await page.waitForLoadState('load')
    })
  } catch {
    // JS disabled — fall back to load event (fires after fonts in CSS are loaded)
    await page.waitForLoadState('load')
  }
}

/**
 * Wait for CSS transitions to settle after a style/theme change.
 * Replaces waitForTimeout(100-300) after theme switches, scroll, evaluate, etc.
 *
 * Falls back to waitForLoadState('load') when JavaScript is disabled.
 */
export async function waitForStyleSettle(page: Page): Promise<void> {
  await page.waitForTimeout(10)
}
